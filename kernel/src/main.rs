#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use bootloader_api::{entry_point, info::FrameBufferInfo, info::PixelFormat, BootInfo};
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

struct Screen {
    buffer: *mut u8,
    info: Option<FrameBufferInfo>,
    active_task: usize,
    completed: [bool; 3],
}

unsafe impl Sync for Screen {}
unsafe impl Send for Screen {}

impl Screen {
    const fn new() -> Self {
        Self {
            buffer: core::ptr::null_mut(),
            info: None,
            active_task: 0,
            completed: [false; 3],
        }
    }

    fn init(&mut self, framebuffer: &mut bootloader_api::info::FrameBuffer) {
        let info = framebuffer.info();
        self.buffer = framebuffer.buffer_mut().as_mut_ptr();
        self.info = Some(info);
        self.fill([10, 18, 32]);
        self.rect(0, 0, info.width, 72, [20, 32, 54]);
        self.text(40, 22, "MICRO-OS", 3, [120, 220, 255]);
        self.text(40, 112, "RUST KERNEL ONLINE", 2, [220, 230, 240]);
        self.text(40, 158, "ROUND ROBIN SCHEDULER", 2, [170, 190, 210]);
        self.text(40, 224, "TASK 1", 2, [255, 190, 90]);
        self.text(340, 224, "TASK 2", 2, [130, 230, 150]);
        self.text(640, 224, "TASK 3", 2, [190, 150, 255]);
        self.rect(40, 270, 240, 26, [44, 64, 86]);
        self.rect(340, 270, 240, 26, [44, 64, 86]);
        self.rect(640, 270, 240, 26, [44, 64, 86]);
        self.text(40, 360, "TIMER 20 HZ", 2, [140, 180, 200]);
        self.text(40, 402, "IDT + PIC + PIT READY", 2, [140, 180, 200]);
        self.text(40, 444, "MEMORY 64K BUMP HEAP", 2, [140, 180, 200]);
    }

    fn schedule(&mut self, task_id: usize, finished: bool) {
        self.active_task = task_id;
        if finished && (1..=3).contains(&task_id) {
            self.completed[task_id - 1] = true;
        }
        let colors = [[255, 190, 90], [130, 230, 150], [190, 150, 255]];
        for (index, color) in colors.iter().enumerate() {
            let x = 40 + index * 300;
            let fill = if self.completed[index] {
                240
            } else if self.active_task == index + 1 {
                180
            } else {
                40
            };
            self.rect(x, 270, fill, 26, *color);
        }
    }

    fn fill(&mut self, color: [u8; 3]) {
        if let Some(info) = self.info {
            self.rect(0, 0, info.width, info.height, color);
        }
    }

    fn rect(&mut self, x: usize, y: usize, width: usize, height: usize, color: [u8; 3]) {
        let Some(info) = self.info else { return };
        let max_x = (x + width).min(info.width);
        let max_y = (y + height).min(info.height);
        for row in y.min(info.height)..max_y {
            for column in x.min(info.width)..max_x {
                self.pixel(column, row, color);
            }
        }
    }

    fn text(&mut self, x: usize, y: usize, text: &str, scale: usize, color: [u8; 3]) {
        for (index, character) in text.bytes().enumerate() {
            self.character(x + index * 6 * scale, y, character, scale, color);
        }
    }

    fn character(&mut self, x: usize, y: usize, character: u8, scale: usize, color: [u8; 3]) {
        let glyph = match character {
            b'A' => [0x0e, 0x11, 0x11, 0x1f, 0x11, 0x11, 0x11],
            b'C' => [0x0f, 0x10, 0x10, 0x10, 0x10, 0x10, 0x0f],
            b'D' => [0x1e, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1e],
            b'E' => [0x1f, 0x10, 0x10, 0x1e, 0x10, 0x10, 0x1f],
            b'I' => [0x1f, 0x04, 0x04, 0x04, 0x04, 0x04, 0x1f],
            b'K' => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
            b'L' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1f],
            b'M' => [0x11, 0x1b, 0x15, 0x15, 0x11, 0x11, 0x11],
            b'N' => [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11],
            b'O' => [0x0e, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0e],
            b'P' => [0x1e, 0x11, 0x11, 0x1e, 0x10, 0x10, 0x10],
            b'R' => [0x1e, 0x11, 0x11, 0x1e, 0x14, 0x12, 0x11],
            b'S' => [0x0f, 0x10, 0x10, 0x0e, 0x01, 0x01, 0x1e],
            b'T' => [0x1f, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
            b'Y' => [0x11, 0x11, 0x0a, 0x04, 0x04, 0x04, 0x04],
            b'0' => [0x0e, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0e],
            b'1' => [0x04, 0x0c, 0x04, 0x04, 0x04, 0x04, 0x1f],
            b'2' => [0x0e, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1f],
            b'3' => [0x1e, 0x01, 0x01, 0x0e, 0x01, 0x01, 0x1e],
            b' ' => [0; 7],
            b'+' => [0x04, 0x04, 0x1f, 0x04, 0x04, 0, 0],
            _ => [0; 7],
        };
        for (row, bits) in glyph.iter().enumerate() {
            for column in 0..5 {
                if bits & (1 << (4 - column)) != 0 {
                    self.rect(x + column * scale, y + row * scale, scale, scale, color);
                }
            }
        }
    }

    fn pixel(&mut self, x: usize, y: usize, color: [u8; 3]) {
        let Some(info) = self.info else { return };
        let offset = (y * info.stride + x) * info.bytes_per_pixel;
        unsafe {
            let destination =
                core::slice::from_raw_parts_mut(self.buffer.add(offset), info.bytes_per_pixel);
            match info.pixel_format {
                PixelFormat::Rgb => destination[..3].copy_from_slice(&color),
                PixelFormat::Bgr => {
                    destination[..3].copy_from_slice(&[color[2], color[1], color[0]])
                }
                PixelFormat::U8 => destination[0] = color[0],
                PixelFormat::Unknown { .. } => destination[..3].copy_from_slice(&color),
                _ => destination[..3].copy_from_slice(&color),
            }
        }
    }
}

static SCREEN: Mutex<Screen> = Mutex::new(Screen::new());

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

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    serial_println!("Micro-OS Rust kernel booted");
    serial_println!("[BOOT] x86_64 bootloader transferred control to kernel");
    serial_println!("[INIT] serial console initialized");

    if let Some(framebuffer) = boot_info.framebuffer.as_mut() {
        SCREEN.lock().init(framebuffer);
    }

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
        SCREEN.lock().schedule(task.id, finished);
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
