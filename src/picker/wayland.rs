// SPDX-License-Identifier: MPL-2.0

//! Entry point for screen capture.
//!
//! This module provides the async `capture_all_outputs()` function that the
//! event loop calls.  It uses the persistent [`CaptureHelper`] to avoid the
//! overhead of creating a fresh Wayland connection per capture (as we did
//! originally).  The helper is created once and reused.
//!
//! The capture flow follows `xdg-desktop-portal-cosmic` exactly:
//!
//! 1. Create a capture session on the persistent connection.
//! 2. Wait for the compositor to send formats (block on condvar).
//! 3. Create a memfd + SHM buffer (Abgr8888).
//! 4. Call `session.capture()` with a full damage rect.
//! 5. Wait for Ready (block on condvar).
//! 6. Read pixels from the memfd via mmap → `RgbaImage`.
//! 7. Build `Handle::from_rgba()` on the capture thread.
//! 8. Return [`CapturedOutput`] with both the handle and the image data.

use std::sync::OnceLock;

use image::RgbaImage;
use tokio::sync::oneshot;

use crate::picker::capture::{CaptureHelper, CaptureSource};
use crate::picker::CapturedOutput;

// ---------------------------------------------------------------------------
// Singleton helper — initialised once and reused for the applet's lifetime.
// ---------------------------------------------------------------------------

fn helper() -> &'static CaptureHelper {
    static HELPER: OnceLock<CaptureHelper> = OnceLock::new();
    HELPER.get_or_init(|| {
        eprintln!("[capture] Initialising persistent CaptureHelper singleton...");
        CaptureHelper::new()
    })
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Capture all connected outputs.
///
/// This is called from the iced event loop via `Task::perform`.  It spawns a
/// dedicated OS thread that uses the persistent [`CaptureHelper`] — no fresh
/// Wayland connection is created per capture.
///
/// Returns the captured outputs with pre-built GPU handles.
pub async fn capture_all_outputs() -> Result<Vec<CapturedOutput>, anyhow::Error> {
    let h = helper();
    let t_start = std::time::Instant::now();
    eprintln!("[capture] === STARTING Wayland screen capture (persistent connection) ===");

    // Read outputs from the helper's state (discovered at init time).
    let wl_outputs = h.outputs();
    let n = wl_outputs.len();
    eprintln!("[capture] {} output(s) from CaptureHelper state", n);

    if n == 0 {
        return Err(anyhow::anyhow!("No Wayland outputs found"));
    }

    // Collect output infos before spawning the thread.
    let output_infos: Vec<_> = wl_outputs
        .iter()
        .map(|o| {
            let info = h.output_info(o);
            (o.clone(), info)
        })
        .collect();

    // Spawn a thread for blocking capture work.
    let (tx, rx) = oneshot::channel();

    std::thread::spawn(move || {
        let captured_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut results: Vec<CapturedOutput> = Vec::with_capacity(n);
            // Capture each output sequentially (same as portal).
            for (output, info) in &output_infos {
                let Some(info) = info else {
                    eprintln!("[capture]   SKIP: no OutputInfo for an output");
                    continue;
                };
                let name = info.name.clone().unwrap_or_default();
                let (ox, oy) = info.location;
                let logical_size = info.logical_size.unwrap_or((0, 0));

                eprintln!("[capture]   Capturing output '{}' ...", name);

                // --- portal's capture_source_shm flow (blocking) ---
                let shm_img = match h.capture_source_shm_blocking(CaptureSource::Output(output.clone())) {
                    Some(img) => img,
                    None => {
                        eprintln!("[capture]   FAILED: capture_source_shm_blocking returned None for '{}'", name);
                        continue;
                    }
                };

                // --- Read pixels via mmap + transform (portal's ShmImage::image_transformed) ---
                let t_read = std::time::Instant::now();
                let rgba: RgbaImage = match shm_img.image_transformed() {
                    Ok(img) => img,
                    Err(e) => {
                        eprintln!("[capture]   FAILED: image_transformed for '{}': {}", name, e);
                        continue;
                    }
                };
                eprintln!(
                    "[capture]   image_transformed for '{}' took {:?} ({}x{})",
                    name,
                    t_read.elapsed(),
                    rgba.width(),
                    rgba.height(),
                );

                // --- Build GPU handle (portal's ScreenshotImage::new) ---
                let t_handle = std::time::Instant::now();
                let handle = cosmic::widget::image::Handle::from_rgba(
                    rgba.width(),
                    rgba.height(),
                    rgba.clone().into_vec(),
                );
                eprintln!(
                    "[capture]   Handle::from_rgba for '{}' took {:?}",
                    name,
                    t_handle.elapsed(),
                );

                results.push(CapturedOutput {
                    name,
                    rgba,
                    image_handle: handle,
                    width: shm_img.width,
                    height: shm_img.height,
                    logical_width: logical_size.0.max(0) as u32,
                    logical_height: logical_size.1.max(0) as u32,
                    pos_x: ox,
                    pos_y: oy,
                });
            }

            Ok(results) as Result<Vec<CapturedOutput>, anyhow::Error>
        }));

        let result = match captured_result {
            Ok(Ok(outputs)) => Ok(outputs),
            Ok(Err(e)) => Err(e),
            Err(panic) => {
                let msg = if let Some(s) = panic.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "unknown panic".to_string()
                };
                eprintln!("[capture] THREAD PANICKED: {msg}");
                Err(anyhow::anyhow!("Capture thread panicked: {msg}"))
            }
        };
        let _ = tx.send(result);
    });

    let result = rx.await.map_err(|_| anyhow::anyhow!("Capture thread was cancelled"))?;
    let captured = result?;

    eprintln!(
        "[capture] === Capture finished: {} output(s) in {:?} ===",
        captured.len(),
        t_start.elapsed(),
    );

    Ok(captured)
}
