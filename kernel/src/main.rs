#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use bootloader_api::{entry_point, BootInfo};
use core::alloc::{GlobalAlloc, Layout};
use core::fmt::{self, Write};
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicUsize, Ordering};
use lazy_static::lazy_static;
use pic8259::ChainedPics;
use spin::Mutex;
use uart_16550::SerialPort;
use x86_64::instructions::port::Port;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

const PIC_1_OFFSET: u8 = 32;
const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;
const TIMER_FREQUENCY_HZ: u32 = 20;
const MAX_TASKS: usize = 8;

entry_point!(kernel_main);

lazy_static! {
    static ref SERIAL1: Mutex<SerialPort> = {
        let mut serial = unsafe { SerialPort::new(0x3F8) };
        serial.init();
        Mutex::new(serial)
    };
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt[InterruptIndex::Timer.as_u8()].set_handler_fn(timer_interrupt_handler);
        idt
    };
}

static PICS: Mutex<ChainedPics> =
    Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

#[derive(Clone, Copy, Eq, PartialEq)]
enum TaskState {
    Ready,
    Running,
    Terminated,
}

#[derive(Clone, Copy)]
struct Task {
    id: usize,
    name: &'static str,
    remaining_steps: usize,
    state: TaskState,
}

impl Task {
    const EMPTY: Option<Task> = None;

    fn new(id: usize, name: &'static str, steps: usize) -> Self {
        Self {
            id,
            name,
            remaining_steps: steps,
            state: TaskState::Ready,
        }
    }

    fn run_one_step(&mut self) -> bool {
        self.state = TaskState::Running;
        self.remaining_steps -= 1;
        let finished = self.remaining_steps == 0;
        self.state = if finished {
            TaskState::Terminated
        } else {
            TaskState::Ready
        };
        finished
    }
}

struct ReadyQueue {
    tasks: [Option<Task>; MAX_TASKS],
    head: usize,
    tail: usize,
    length: usize,
    switches: usize,
    completed: usize,
}

impl ReadyQueue {
    const fn new() -> Self {
        Self {
            tasks: [Task::EMPTY; MAX_TASKS],
            head: 0,
            tail: 0,
            length: 0,
            switches: 0,
            completed: 0,
        }
    }

    fn push(&mut self, task: Task) {
        if self.length == MAX_TASKS {
            panic!("ready queue is full");
        }
        self.tasks[self.tail] = Some(task);
        self.tail = (self.tail + 1) % MAX_TASKS;
        self.length += 1;
    }

    fn pop(&mut self) -> Option<Task> {
        let task = self.tasks[self.head].take()?;
        self.head = (self.head + 1) % MAX_TASKS;
        self.length -= 1;
        Some(task)
    }

    fn run_one_step(&mut self) -> Option<(Task, bool)> {
        let mut task = self.pop()?;
        self.switches += 1;
        let finished = task.run_one_step();
        if finished {
            self.completed += 1;
        } else {
            self.push(task);
        }
        Some((task, finished))
    }

    fn is_empty(&self) -> bool {
        self.length == 0
    }
}

static READY_QUEUE: Mutex<ReadyQueue> = Mutex::new(ReadyQueue::new());
static SHARED_COUNTER: Mutex<usize> = Mutex::new(0);

struct BumpAllocator {
    next: AtomicUsize,
}

const HEAP_SIZE: usize = 64 * 1024;
static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let heap_start = core::ptr::addr_of_mut!(HEAP) as *mut u8 as usize;
        let current = self.next.load(Ordering::Relaxed);
        let aligned = (heap_start + current + layout.align() - 1) & !(layout.align() - 1);
        let offset = aligned - heap_start;
        let end = match offset.checked_add(layout.size()) {
            Some(end) if end <= HEAP_SIZE => end,
            _ => return core::ptr::null_mut(),
        };
        if self
            .next
            .compare_exchange(current, end, Ordering::SeqCst, Ordering::Relaxed)
            .is_ok()
        {
            aligned as *mut u8
        } else {
            core::ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[global_allocator]
static ALLOCATOR: BumpAllocator = BumpAllocator {
    next: AtomicUsize::new(0),
};

#[derive(Clone, Copy)]
#[repr(u8)]
enum InterruptIndex {
    Timer = PIC_1_OFFSET,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }
}

fn kernel_main(_boot_info: &'static mut BootInfo) -> ! {
    serial_println!("Micro-OS Rust kernel booted");
    serial_println!("[BOOT] x86_64 bootloader transferred control to kernel");
    serial_println!("[INIT] serial console initialized");

    IDT.load();
    unsafe { PICS.lock().initialize() };
    configure_timer(TIMER_FREQUENCY_HZ);
    x86_64::instructions::interrupts::enable();
    serial_println!("[INIT] IDT, PIC and PIT timer initialized");

    {
        let mut queue = READY_QUEUE.lock();
        queue.push(Task::new(1, "shell", 3));
        queue.push(Task::new(2, "logger", 2));
        queue.push(Task::new(3, "worker", 4));
    }
    serial_println!("[TASK] created shell, logger and worker");

    let allocation = unsafe { ALLOCATOR.alloc(Layout::from_size_align(128, 8).unwrap()) };
    if allocation.is_null() {
        serial_println!("[MEM] allocation failed");
    } else {
        serial_println!("[MEM] allocated 128 bytes from kernel bump heap");
    }

    serial_println!("[RUN] timer-driven Round Robin scheduling started");
    hlt_loop();
}

fn configure_timer(frequency_hz: u32) {
    let divisor = 1_193_182 / frequency_hz;
    let mut command = Port::<u8>::new(0x43);
    let mut channel = Port::<u8>::new(0x40);
    unsafe {
        command.write(0x36);
        channel.write((divisor & 0xff) as u8);
        channel.write((divisor >> 8) as u8);
    }
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    let mut counter = SHARED_COUNTER.lock();
    *counter += 1;
    let tick = *counter;
    drop(counter);

    let mut queue = READY_QUEUE.lock();
    if let Some((task, finished)) = queue.run_one_step() {
        serial_println!(
            "[SCHED] tick={} task={} ({}){}",
            tick,
            task.id,
            task.name,
            if finished { " TERMINATED" } else { "" }
        );
        if queue.is_empty() {
            serial_println!(
                "[DONE] switches={} completed_tasks={}",
                queue.switches,
                queue.completed
            );
            exit_qemu(0x10);
            x86_64::instructions::interrupts::disable();
        }
    }
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8())
    };
}

struct SerialWriter;

fn exit_qemu(code: u32) {
    let mut port = Port::<u32>::new(0xf4);
    unsafe {
        port.write(code);
    }
}

impl Write for SerialWriter {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        SERIAL1.lock().write_str(text)
    }
}

#[macro_export]
macro_rules! serial_print { ($($arg:tt)*) => {{ use core::fmt::Write; let _ = write!($crate::SerialWriter, $($arg)*); }}; }

#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(concat!($fmt, "\n"), $($arg)*));
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("[PANIC] {}", info);
    hlt_loop();
}

fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
