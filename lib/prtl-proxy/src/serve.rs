use crate::PrtlService;
use futures_util::stream::StreamExt;
use prtl_messages::{BusMessage, RegisterProxyRequest};
use std::sync::Arc;

pub async fn serve(service: Arc<dyn PrtlService>) -> Result<(), Box<dyn std::error::Error>> {
    let nats_addr = std::env::var("NATS_ADDR").unwrap_or_else(|_| "nats://localhost:4222".into());
    let nc = async_nats::connect(&nats_addr).await?;

    let descriptor = service.descriptor();

    let register_subject = BusMessage::subject_for_register(&descriptor.service_name);
    let subject = BusMessage::subject_for_rpc(&descriptor.service_name);

    // Register on startup
    tracing::info!("Registering proxy with NATS");
    let register_payload = rmp_serde::to_vec_named(&BusMessage::RegisterParser(RegisterProxyRequest {
        descriptor: descriptor.clone(),
    }))?;
    nc.publish(register_subject.clone(), register_payload.clone().into())
        .await?;

    // Listen for discovery requests
    let nc_discovery = nc.clone();
    let register_payload_clone = register_payload.clone();
    let register_subject_clone = register_subject.clone();
    tokio::spawn(async move {
        let discovery_subject = BusMessage::subject_for_discovery();
        let mut discovery_sub = match nc_discovery.subscribe(discovery_subject.clone()).await {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to subscribe to discovery: {}", e);
                return;
            }
        };

        tracing::info!("Listening for discovery requests on {}", discovery_subject);

        while let Some(_msg) = discovery_sub.next().await {
            tracing::info!("Received discovery request, re-registering proxy");
            if let Err(e) = nc_discovery
                .publish(register_subject_clone.clone(), register_payload_clone.clone().into())
                .await
            {
                tracing::error!("Failed to re-register proxy: {}", e);
            }
        }
    });

    tracing::info!("Listening on NATS subject: {}", subject);

    let mut subscription = nc.subscribe(subject).await?;

    while let Some(msg) = subscription.next().await {
        let service = service.clone();
        let nc = nc.clone();

        tokio::spawn(async move {
            let reply_subject = match msg.reply {
                Some(s) => s,
                None => return,
            };

            let bus_msg: BusMessage = match rmp_serde::from_slice(&msg.payload) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("Failed to deserialize request: {}", e);
                    return;
                }
            };

            let response = match bus_msg {
                BusMessage::ProxyRequest(request) => match service.handle_request(request).await {
                    Ok(resp) => BusMessage::ProxyResponse(resp),
                    Err(e) => {
                        let resp = http::Response::builder()
                            .status(500)
                            .body(e.to_string().into_bytes())
                            .unwrap();
                        BusMessage::ProxyResponse(resp)
                    }
                },
                _ => {
                    eprintln!("Unexpected message type");
                    return;
                }
            };

            let payload = match rmp_serde::to_vec(&response) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Failed to serialize response: {}", e);
                    return;
                }
            };

            if let Err(e) = nc.publish(reply_subject, payload.into()).await {
                eprintln!("Failed to send reply: {}", e);
            }
        });
    }

    Ok(())
}
