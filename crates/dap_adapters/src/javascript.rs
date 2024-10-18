use crate::*;

#[derive(Debug, Eq, PartialEq, Clone)]
pub(crate) struct JsDebugAdapter {}

impl JsDebugAdapter {
    const ADAPTER_NAME: &'static str = "vscode-js-debug";
    const ADAPTER_PATH: &'static str = "src/dapDebugServer.js";

    pub(crate) fn new() -> Self {
        JsDebugAdapter {}
    }
}

#[async_trait(?Send)]
impl DebugAdapter for JsDebugAdapter {
    fn name(&self) -> DebugAdapterName {
        DebugAdapterName(Self::ADAPTER_NAME.into())
    }

    async fn connect(
        &self,
        adapter_binary: &DebugAdapterBinary,
        cx: &mut AsyncAppContext,
    ) -> Result<(TransportParams, Option<AdapterLogIo>)> {
        let host = TCPHost {
            port: Some(8133),
            host: None,
            delay: Some(1000),
        };

        create_tcp_client(host, adapter_binary, cx)
            .await
            .map(|(transport, log_io)| (transport, Some(log_io)))
    }

    async fn fetch_binary(
        &self,
        delegate: &dyn DapDelegate,
        _: &DebugAdapterConfig,
    ) -> Result<DebugAdapterBinary> {
        let node_runtime = delegate
            .node_runtime()
            .ok_or(anyhow!("Couldn't get npm runtime"))?;

        let adapter_path = paths::debug_adapters_dir().join(self.name());

        Ok(DebugAdapterBinary {
            command: node_runtime
                .binary_path()
                .await?
                .to_string_lossy()
                .into_owned(),
            arguments: Some(vec![
                adapter_path.join(Self::ADAPTER_PATH).into(),
                "8133".into(),
            ]),
            envs: None,
        })
    }

    async fn install_binary(&self, delegate: &dyn DapDelegate) -> Result<()> {
        let adapter_path = paths::debug_adapters_dir().join(self.name());
        let fs = delegate.fs();

        if fs.is_dir(adapter_path.as_path()).await {
            return Ok(());
        }

        if let Some(http_client) = delegate.http_client() {
            if !adapter_path.exists() {
                fs.create_dir(&adapter_path.as_path()).await?;
            }

            let release = latest_github_release(
                "microsoft/vscode-js-debug",
                false,
                false,
                http_client.clone(),
            )
            .await?;

            let asset_name = format!("{}-{}", self.name(), release.tag_name);
            let zip_path = adapter_path.join(asset_name);

            if fs::metadata(&zip_path).await.is_err() {
                let mut response = http_client
                    .get(&release.zipball_url, Default::default(), true)
                    .await
                    .context("Error downloading release")?;

                let mut file = File::create(&zip_path).await?;
                futures::io::copy(response.body_mut(), &mut file).await?;

                let _unzip_status = process::Command::new("unzip")
                    .current_dir(&adapter_path)
                    .arg(&zip_path)
                    .output()
                    .await?
                    .status;

                let mut ls = process::Command::new("ls")
                    .current_dir(&adapter_path)
                    .stdout(Stdio::piped())
                    .spawn()?;

                let std = ls
                    .stdout
                    .take()
                    .ok_or(anyhow!("Failed to list directories"))?
                    .into_stdio()
                    .await?;

                let file_name = String::from_utf8(
                    process::Command::new("grep")
                        .arg("microsoft-vscode-js-debug")
                        .stdin(std)
                        .output()
                        .await?
                        .stdout,
                )?;

                let file_name = file_name.trim_end();

                process::Command::new("sh")
                    .current_dir(&adapter_path)
                    .arg("-c")
                    .arg(format!("mv {file_name}/* ."))
                    .output()
                    .await?;

                process::Command::new("rm")
                    .current_dir(&adapter_path)
                    .arg("-rf")
                    .arg(file_name)
                    .arg(zip_path)
                    .output()
                    .await?;

                let _ = delegate
                    .node_runtime()
                    .ok_or(anyhow!("Couldn't get npm runtime"))?
                    .run_npm_subcommand(&adapter_path, "install", &[])
                    .await
                    .ok();

                let _ = delegate
                    .node_runtime()
                    .ok_or(anyhow!("Couldn't get npm runtime"))?
                    .run_npm_subcommand(&adapter_path, "run", &["compile"])
                    .await
                    .ok();

                return Ok(());
            }
        }

        bail!("Install or fetch not implemented for Javascript debug adapter (yet)");
    }

    fn request_args(&self, config: &DebugAdapterConfig) -> Value {
        json!({
            "program": config.program,
            "type": "pwa-node",
        })
    }
}
