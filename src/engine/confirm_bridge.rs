use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

/// UI와 엔진 사이에서 컨펌 응답을 중계하는 헬퍼이다.
#[derive(Clone, Debug)]
pub struct ConfirmBridge {
    /// 내부 상태를 보관한다.
    inner: Arc<ConfirmBridgeInner>,
}

/// ConfirmBridge 내부 구현체이다.
#[derive(Debug)]
struct ConfirmBridgeInner {
    /// 다음 요청 ID를 생성하기 위한 카운터이다.
    next_id: AtomicU64,
    /// 대기 중인 요청과 응답 채널을 매핑한다.
    pending: Mutex<HashMap<u64, oneshot::Sender<bool>>>,
}

impl ConfirmBridge {
    /// 상호작용 가능한 브리지를 생성한다.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(ConfirmBridgeInner {
                next_id: AtomicU64::new(1),
                pending: Mutex::new(HashMap::new()),
            }),
        }
    }

    /// 새로운 컨펌 요청을 등록하고 request_id 및 Receiver를 반환한다.
    pub fn register(&self) -> (u64, oneshot::Receiver<bool>) {
        let request_id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        self.inner
            .pending
            .lock()
            .expect("ConfirmBridge mutex poisoned")
            .insert(request_id, tx);
        (request_id, rx)
    }

    /// 지정한 요청 ID로 응답을 전송한다.
    pub fn respond(&self, request_id: u64, accepted: bool) -> bool {
        if let Some(sender) = self
            .inner
            .pending
            .lock()
            .expect("ConfirmBridge mutex poisoned")
            .remove(&request_id)
        {
            let _ = sender.send(accepted);
            true
        } else {
            false
        }
    }

    /// 대기 중인 요청을 취소하고 맵에서 제거한다.
    pub fn cancel(&self, request_id: u64) {
        self.inner
            .pending
            .lock()
            .expect("ConfirmBridge mutex poisoned")
            .remove(&request_id);
    }
}
