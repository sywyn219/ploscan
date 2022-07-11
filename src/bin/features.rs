use futures;
use std::{future::Future, pin::Pin, sync::{Arc, Mutex}, task::{Context, Poll, Waker}, thread, time::Duration};
use crossbeam_channel;

fn main() {
    // futures::executor::Spawn(TimerFuture::);
}





struct SharedState {
    completed: bool,
    waker: Option<Waker>,
}

pub struct TimerFuture {
    share_state: Arc<Mutex<SharedState>>,
}

impl Future for TimerFuture {
    type Output = ();
    // executor will run this poll ,and Context is to tell future how to wakeup the task.
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut share_state = self.share_state.lock().unwrap();
        if share_state.completed {
            println!("future ready. execute poll to return.");
            Poll::Ready(())
        } else {
            println!("future not ready, tell the future task how to wakeup to executor");
            // 你要告诉future，当事件就绪后怎么唤醒任务去调度执行，而这个waker根具体的调度器有关
            // 调度器执行的时候会将上下文信息传进来，里面最重要的一项就是Waker
            share_state.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}