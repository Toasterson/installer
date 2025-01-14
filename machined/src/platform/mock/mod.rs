use crate::util::report_install_debug;
use crate::ProgressMessage;
use machineconfig::MachineConfig;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::Sender;

pub async fn install_system(
    mc: &MachineConfig,
    tx: Sender<ProgressMessage>,
) -> Result<(), SendError<ProgressMessage>> {
    tx.send(report_install_debug("Mocking Installation"))
        .await?;
    for pool in &mc.pools {
        tx.send(report_install_debug(
            format!(
                "Would create pool {} with vdevs {} and compression {}",
                pool.name,
                pool.vdevs
                    .iter()
                    .map(|vdev| format!("\"{}: {}\"", vdev.kind, vdev.disks.join(",")))
                    .collect::<Vec<_>>()
                    .join(","),
                pool.compression
            )
            .as_str(),
        ))
        .await?;
    }

    tx.send(report_install_debug(
        format!("Would extract image {} as root", &mc.image).as_str(),
    ))
    .await?;

    tx.send(report_install_debug(
        format!("Would set Hostname to {}", &mc.hostname).as_str(),
    ))
    .await?;

    for ns in &mc.nameservers {
        tx.send(report_install_debug(
            format!("Would add nameserver {}", ns).as_str(),
        ))
        .await?;
    }

    for (idx, iface) in mc.interfaces.iter().enumerate() {
        if let Some(selector) = &iface.selector {
            //TODO: Add some search code in multiple platforms
            tx.send(report_install_debug(
                format!("Would work on interface {}", selector).as_str(),
            ))
            .await?;
        } else {
            tx.send(report_install_debug(
                format!("Would operate on {} interface", idx).as_str(),
            ))
            .await?;
        }

        if let Some(name) = &iface.name {
            tx.send(report_install_debug(
                format!("Would set interface name to {}", name).as_str(),
            ))
            .await?;
        }
    }

    Ok(())
}
