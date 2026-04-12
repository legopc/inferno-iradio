//! Auto-configure ~/.asoundrc with `pcm.inferno_iradio_N` blocks.
//! Reads BIND_IP, DEVICE_ID, CLOCK_PATH and NAME from the existing Inferno
//! PCM entries and appends one block per player slot.
//! No-op if blocks already exist.

use anyhow::{Context, Result};
use std::path::Path;

/// Append `pcm.inferno_iradio_N` blocks for slots 1..=max_slots.
pub fn ensure_iradio_alsa(max_slots: usize, asoundrc_path: &Path) -> Result<()> {
    let content = std::fs::read_to_string(asoundrc_path).unwrap_or_default();

    if content.contains("pcm.inferno_iradio_1") {
        tracing::debug!("iradio ALSA blocks already present in {:?}", asoundrc_path);
        return Ok(());
    }

    let bind_ip = extract_field(&content, "BIND_IP")
        .context("BIND_IP not found in .asoundrc — is the Inferno appliance configured?")?;
    let base_id =
        extract_field(&content, "DEVICE_ID").context("DEVICE_ID not found in .asoundrc")?;
    let clock_path =
        extract_field(&content, "CLOCK_PATH").unwrap_or_else(|| "/tmp/ptp-usrvclock".to_string());
    // Take only the first NAME value (the Dante device base name)
    let base_name = extract_field(&content, "NAME").unwrap_or_else(|| "Inferno".to_string());

    // DEVICE_ID prefix = first 12 hex chars; iradio offsets start at 10 (0x000a)
    let id_prefix = if base_id.len() >= 12 {
        base_id[..12].to_string()
    } else {
        base_id.clone()
    };
    let base_suffix = u32::from_str_radix(
        if base_id.len() >= 4 {
            &base_id[base_id.len() - 4..]
        } else {
            "0"
        },
        16,
    )
    .unwrap_or(0);

    let mut blocks = String::new();
    for slot in 1..=max_slots {
        let process_id = 9 + slot; // slot 1 → PID 10, slot 2 → PID 11 …
        let alt_port = 6100 + (slot - 1) * 20; // 6100, 6120, 6140, 6160
        let device_id = format!(
            "{}{:04x}",
            id_prefix,
            base_suffix + 9 + slot as u32 // offset 10..13 from base
        );
        let dante_name = format!("{}-ir{}", base_name, slot);

        blocks.push_str(&format!(
            "\n# iradio-bridge Dante TX slot {slot}\npcm.inferno_iradio_{slot} {{\n    type inferno\n    NAME \"{dante_name}\"\n    BIND_IP {bind_ip}\n    SAMPLE_RATE 48000\n    PROCESS_ID {process_id}\n    ALT_PORT {alt_port}\n    RX_CHANNELS 0\n    TX_CHANNELS 2\n    TX_LATENCY_NS 10000000\n    RX_LATENCY_NS 10000000\n    CLOCK_PATH {clock_path}\n    DEVICE_ID {device_id}\n    hint {{ show off description \"Inferno iradio slot {slot} TX\" }}\n}}\n"
        ));
    }

    let new_content = format!("{}\n{}", content.trim_end(), blocks);
    std::fs::write(asoundrc_path, new_content).context("Failed to write .asoundrc")?;

    tracing::info!(
        "Wrote {} iradio ALSA PCM blocks to {:?}",
        max_slots,
        asoundrc_path
    );
    Ok(())
}

fn extract_field(content: &str, field: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(field) {
            let val = rest.trim().trim_matches('"');
            if !val.is_empty() {
                return Some(val.to_string());
            }
        }
    }
    None
}
