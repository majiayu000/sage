use std::sync::Arc;

use crate::agent::{ExecutionOptions, UnifiedExecutor};
use crate::config::Config;
use crate::error::SageResult;
use crate::input::InputChannel;
use crate::llm::messages::LlmMessage;
use crate::output::OutputMode;
use crate::runtime::{
    RuntimeControlResult, RuntimeForkRequest, RuntimeInterruptRequest, RuntimeOperation,
    RuntimeProtocolStream, RuntimeRunResult, RuntimeStartRequest, RuntimeStateCapabilities,
    RuntimeStatus, boxed_runtime_unsupported_error, boxed_runtime_validation_error,
};
use crate::runtime_protocol::{RuntimeResponse, RuntimeSource};
use crate::session::{JsonlSessionStorage, SessionMetadata};
use crate::skills::SkillRegistry;
use crate::thread_store::{ThreadStore, ThreadStoreError};
use crate::tools::Tool;
use crate::trajectory::SessionRecorder;
use crate::types::TaskMetadata;
use tokio::sync::{Mutex, RwLock};

pub struct Runtime {
    config: Config,
    options: ExecutionOptions,
    source: RuntimeSource,
    thread_store: Option<Arc<dyn ThreadStore>>,
    protocol_stream: RuntimeProtocolStream,
}

impl Runtime {
    pub fn new(config: Config, options: ExecutionOptions) -> Self {
        Self {
            config,
            options,
            source: RuntimeSource::Runtime,
            thread_store: None,
            protocol_stream: RuntimeProtocolStream::default(),
        }
    }

    pub fn with_source(mut self, source: RuntimeSource) -> Self {
        self.source = source;
        self
    }

    pub fn with_thread_store(mut self, store: Arc<dyn ThreadStore>) -> Self {
        self.thread_store = Some(store);
        self
    }

    pub fn with_protocol_stream(mut self, stream: RuntimeProtocolStream) -> Self {
        self.protocol_stream = stream;
        self
    }

    pub fn state_capabilities(&self) -> RuntimeStateCapabilities {
        if self.thread_store.is_some() {
            RuntimeStateCapabilities::thread_store()
        } else {
            RuntimeStateCapabilities::ephemeral()
        }
    }

    pub fn start_request(
        &self,
        task_description: impl Into<String>,
        working_dir: impl ToString,
    ) -> RuntimeStartRequest {
        RuntimeStartRequest::new(
            TaskMetadata::new(task_description.into(), working_dir.to_string()),
            self.source,
        )
    }

    pub fn build_executor(&self) -> SageResult<RuntimeExecutor> {
        let executor = UnifiedExecutor::with_options(self.config.clone(), self.options.clone())?;
        Ok(RuntimeExecutor {
            inner: executor,
            source: self.source,
            state: self.state_capabilities(),
            protocol_stream: self.protocol_stream,
        })
    }

    pub fn resume(
        &self,
        request: crate::runtime::RuntimeResumeRequest,
    ) -> RuntimeControlResult<RuntimeResponse> {
        Err(boxed_runtime_unsupported_error(
            RuntimeOperation::Resume,
            self.source,
            format!(
                "runtime resume for thread {} is not supported by the current execution loop",
                request.thread_id
            ),
        ))
    }

    pub fn fork(&self, _request: RuntimeForkRequest) -> RuntimeControlResult<RuntimeResponse> {
        Err(boxed_runtime_unsupported_error(
            RuntimeOperation::Fork,
            self.source,
            "runtime fork is not supported by the current execution loop",
        ))
    }

    pub fn interrupt(
        &self,
        _request: RuntimeInterruptRequest,
    ) -> RuntimeControlResult<RuntimeResponse> {
        Err(boxed_runtime_unsupported_error(
            RuntimeOperation::Interrupt,
            self.source,
            "runtime interrupt is not supported by the current execution loop",
        ))
    }

    pub async fn status(&self, thread_id: &str) -> RuntimeControlResult<RuntimeStatus> {
        let Some(store) = &self.thread_store else {
            return Err(boxed_runtime_unsupported_error(
                RuntimeOperation::Status,
                self.source,
                "runtime status requires a ThreadStore",
            ));
        };

        let snapshot = store.read_thread(thread_id).await.map_err(|err| {
            boxed_runtime_validation_error(
                RuntimeOperation::Status,
                self.source,
                thread_store_status_error_code(&err),
                format!("runtime status could not read thread: {err}"),
            )
        })?;

        Ok(RuntimeStatus {
            thread_id: snapshot.thread.thread_id,
            state: RuntimeStateCapabilities::thread_store(),
            thread_status: snapshot.thread.status,
            turn_count: snapshot.turns.len(),
            item_count: snapshot.items.len(),
        })
    }
}

fn thread_store_status_error_code(err: &ThreadStoreError) -> &'static str {
    match err {
        ThreadStoreError::ThreadNotFound(_) => "thread_not_found",
        ThreadStoreError::InvalidInput(_) => "invalid_thread_id",
        _ => "thread_store_error",
    }
}

pub struct RuntimeExecutor {
    inner: UnifiedExecutor,
    source: RuntimeSource,
    state: RuntimeStateCapabilities,
    protocol_stream: RuntimeProtocolStream,
}

impl RuntimeExecutor {
    pub async fn start(&mut self, request: RuntimeStartRequest) -> SageResult<RuntimeRunResult> {
        let outcome = self.inner.execute(request.task.clone()).await?;
        let notifications =
            self.protocol_stream
                .notifications_for_result(&request, &outcome, self.source);
        Ok(RuntimeRunResult::new(
            request,
            outcome,
            notifications,
            self.state.clone(),
            self.source,
        ))
    }

    pub async fn start_task(&mut self, task: TaskMetadata) -> SageResult<RuntimeRunResult> {
        self.start(RuntimeStartRequest::new(task, self.source))
            .await
    }

    pub fn register_tools(&mut self, tools: Vec<Arc<dyn Tool>>) {
        self.inner.register_tools(tools);
    }

    pub fn set_output_mode(&mut self, mode: OutputMode) {
        self.inner.set_output_mode(mode);
    }

    pub fn set_input_channel(&mut self, channel: InputChannel) {
        self.inner.set_input_channel(channel);
    }

    pub fn set_session_recorder(&mut self, recorder: Arc<Mutex<SessionRecorder>>) {
        self.inner.set_session_recorder(recorder);
    }

    pub fn set_jsonl_storage(&mut self, storage: Arc<JsonlSessionStorage>) {
        self.inner.set_jsonl_storage(storage);
    }

    pub async fn enable_session_recording(&mut self) -> SageResult<String> {
        self.inner.enable_session_recording().await
    }

    pub fn init_subagent_support(&self) -> SageResult<()> {
        self.inner.init_subagent_support()
    }

    pub fn skill_registry(&self) -> Arc<RwLock<SkillRegistry>> {
        self.inner.skill_registry()
    }

    pub fn options(&self) -> &ExecutionOptions {
        self.inner.options()
    }

    pub async fn restore_session(&mut self, session_id: &str) -> SageResult<Vec<LlmMessage>> {
        self.inner.restore_session(session_id).await
    }

    pub async fn get_most_recent_session(&self) -> SageResult<Option<SessionMetadata>> {
        self.inner.get_most_recent_session().await
    }

    pub fn switch_model(&mut self, model: &str) -> SageResult<String> {
        self.inner.switch_model(model)
    }
}
