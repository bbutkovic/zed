use anyhow::Result;
use async_trait::async_trait;
use task::DebugAdapterConfig;

use crate::*;

#[derive(Debug, Eq, PartialEq, Clone)]
pub(crate) struct LldbDebugAdapter {}

impl LldbDebugAdapter {
    const ADAPTER_NAME: &'static str = "lldb";

    pub(crate) fn new() -> Self {
        LldbDebugAdapter {}
    }
}

#[async_trait(?Send)]
impl DebugAdapter for LldbDebugAdapter {
    fn name(&self) -> DebugAdapterName {
        DebugAdapterName(Self::ADAPTER_NAME.into())
    }

    async fn connect(
        &self,
        adapter_binary: &DebugAdapterBinary,
        _: &mut AsyncAppContext,
    ) -> Result<(TransportParams, Option<AdapterLogIo>)> {
        create_stdio_client(adapter_binary).map(|transport| (transport, None))
    }

    async fn install_binary(&self, _: &dyn DapDelegate) -> Result<()> {
        bail!("Install or fetch not implemented for lldb debug adapter (yet)")
    }

    async fn fetch_binary(
        &self,
        _: &dyn DapDelegate,
        _: &DebugAdapterConfig,
    ) -> Result<DebugAdapterBinary> {
        bail!("Install or fetch not implemented for lldb debug adapter (yet)")
    }

    fn request_args(&self, config: &DebugAdapterConfig) -> Value {
        json!({"program": config.program})
    }
}
