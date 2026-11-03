from pathlib import Path
import re
import sys

SCHEDULER = re.compile(r"\[SCHED\] tick=(\d+) task=(\d+) \(([^)]+)\)")


def main() -> None:
    path = Path(sys.argv[1] if len(sys.argv) > 1 else "traces/demo.log")
    text = path.read_text(encoding="utf-8")
    events = SCHEDULER.findall(text)
    if not events:
        raise SystemExit(f"No scheduler events found in {path}")

    task_counts = {}
    for _, task_id, _ in events:
        task_counts[task_id] = task_counts.get(task_id, 0) + 1

    print(f"trace: {path}")
    print(f"timer_ticks: {events[-1][0]}")
    print(f"scheduler_switches: {len(events)}")
    print(f"task_turns: {task_counts}")
    print(f"fairness_range: {max(task_counts.values()) - min(task_counts.values())}")


if __name__ == "__main__":
    main()
