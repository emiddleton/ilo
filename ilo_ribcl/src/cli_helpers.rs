#[macro_export]
macro_rules! parse_node_auth {
    ($opt:ident) => {{
        use anyhow::Context;
        use ilo_console::ilo2::auth::Auth;
        use ilo_ribcl::{
            client::{Node, ProxyClient},
            commands,
            types::FwVersion,
        };
        use std::{boxed::Box, fs};

        let endpoint_filename = $opt.endpoint.as_path().display().to_string();

        let endpoint_json = fs::read_to_string(&$opt.endpoint)
            .with_context(|| format!("missing or invalid endpoint file {}", endpoint_filename))?;
        let mut node: Node = Node::from_json(&endpoint_json).await?;

        if !$opt.no_update {
            serde_json::to_writer_pretty(
                &fs::File::create(&$opt.endpoint)
                    .with_context(|| format!("couldn't open or create {}", endpoint_filename))?,
                &node,
            )
            .with_context(|| format!("couldn't write config to {}", endpoint_filename))?;
        };

        let mut node = if $opt.proxy_cache {
            let auth = node.auth();
            let firmware = node.firmware().unwrap();
            let client = Node::client_from_auth_and_fw(&auth, &firmware).unwrap();
            let proxy_client = Box::new(ProxyClient::new(auth.clone(), firmware.clone(), client));
            Node::new_with_fw_and_client(auth, firmware, proxy_client)
        } else {
            node
        };

        node
    }};
}
