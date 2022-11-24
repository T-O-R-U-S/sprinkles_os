use core::task::{Poll};
use core::{future::Future, pin::Pin, task::Context};
use core::sync::atomic::{AtomicU64, Ordering};

use alloc::boxed::Box;

pub struct Task {
    id: TaskId,
    future: Pin<Box<dyn Future<Output = ()>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct TaskId(u64);

impl TaskId {
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        TaskId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}


impl Task {
    pub fn new(future: impl Future<Output = ()> + 'static) -> Task {
        Task {
            id: TaskId::new(),
            future: Box::pin(future),
        }
    }

    fn poll(&mut self, context: &mut Context) -> Poll<()> {
        self.future.as_mut().poll(context)
    }
}

pub mod simple_executor {
    use super::Task;
    use alloc::collections::VecDeque;
    use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

    pub struct SimpleExecutor {
        task_queue: VecDeque<Task>,
    }

    impl SimpleExecutor {
        pub fn new() -> Self {
            SimpleExecutor {
                task_queue: VecDeque::new(),
            }
        }

        pub fn spawn(&mut self, task: Task) {
            self.task_queue.push_back(task)
        }

        pub fn run(&mut self) {
            while let Some(mut task) = self.task_queue.pop_front() {
                let waker = dummy_waker();
                let mut context = Context::from_waker(&waker);
                match task.poll(&mut context) {
                    Poll::Ready(()) => {}
                    Poll::Pending => self.task_queue.push_back(task),
                }
            }
        }
    }

    fn dummy_raw_waker() -> RawWaker {
        fn no_op(_: *const ()) {}
        fn clone(_: *const ()) -> RawWaker {
            dummy_raw_waker()
        }

        // TODO: Impl rest of operations
        let vtable = &RawWakerVTable::new(clone, no_op, no_op, no_op);
        RawWaker::new(0 as *const (), vtable)
    }

    fn dummy_waker() -> Waker {
        unsafe { Waker::from_raw(dummy_raw_waker()) }
    }
}

pub mod executor {
    use super::{Task, TaskId};

    use alloc::{collections::BTreeMap, sync::Arc, task::Wake};
    use core::task::{Waker, Context, Poll};
    use crossbeam::queue::ArrayQueue;

    pub struct Executor {
        tasks: BTreeMap<TaskId, Task>,
        task_queue: Arc<ArrayQueue<TaskId>>,
        waker_cache: BTreeMap<TaskId, Waker>
    }

    impl Executor {
        pub fn new() -> Self {
            Executor {
                tasks: BTreeMap::new(),
                task_queue: Arc::new(ArrayQueue::new(100)),
                waker_cache: BTreeMap::new()
            }
        }

        pub fn spawn(&mut self, task: Task) {
            let task_id = task.id;

            if self.tasks.insert(task.id, task).is_some() {
                unreachable!("Task with same ID already in tasks");
            }

            self.task_queue.push(task_id).expect("Task queue full.")
        }

        pub fn run(&mut self) -> ! {
            loop {
                self.run_ready_tasks();
                self.sleep_if_idle();
            }
        }

        fn sleep_if_idle(&self) {
            if self.task_queue.is_empty() {
                x86_64::instructions::hlt();
            }
        }

        fn run_ready_tasks(&mut self) {
            let Self {
                tasks,
                task_queue,
                waker_cache
            } = self;

            while let Some(task_id) = task_queue.pop() {
                let task = match tasks.get_mut(&task_id) {
                    Some(task) => task,
                    None => continue
                };

                let waker = waker_cache.entry(task_id).or_insert_with(|| TaskWaker::new(task_id, task_queue.clone()));

                let mut context = Context::from_waker(waker);

                match task.poll(&mut context) {
                    Poll::Ready(()) => {
                        tasks.remove(&task_id);
                        waker_cache.remove(&task_id);
                    }
                    Poll::Pending => {}
                }
            }
        }
    }

    struct TaskWaker {
        task_id: TaskId,
        task_queue: Arc<ArrayQueue<TaskId>>,
    }

    impl TaskWaker {
        fn new(task_id: TaskId, task_queue: Arc<ArrayQueue<TaskId>>) -> Waker {
            Waker::from(Arc::new(TaskWaker {
                task_id,
                task_queue
            }))
        }

        fn wake_task(&self) {
            self.task_queue.push(self.task_id).expect("Task queue full.")
        }
    }

    impl Wake for TaskWaker {
        fn wake(self: Arc<Self>) {
            self.wake_task();
        }

        fn wake_by_ref(self: &Arc<Self>) {
            self.wake_task();
        }
    }
}