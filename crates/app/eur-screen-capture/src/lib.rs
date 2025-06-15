use scap::Target;
use scap::capturer::Capturer;

struct ScreenRecorder {
    target: Target,
    recorder: Capturer,
}

impl ScreenRecorder {
    pub fn new(target: Target, recorder: Capturer) -> Self {
        Self { target, recorder }
    }
}

#[test]
fn record() {
    let width = 1920;
    let height = 1080;
    let framerate = 1;
    // let num_frames = 60;
    let output_dir = "output";

    let args = [
        "-f",
        "rawvideo",
        "-pix_fmt",
        "rgb24",
        "-s",
        &format!("{}x{}", width, height),
        "-r",
        &framerate.to_string(),
        "-i",
        "pipe:0", // read from stdin
        "-c:v",
        "libx265",
        "-tag:v",
        "hvc1",
        "-preset",
        "ultrafast",
        "-crf",
        "23",
        "-pix_fmt",
        "yuv420p",
        &format!("{}/render.mp4", output_dir),
    ];

    //[
    //     "-f",
    //     "rawvideo",
    //     "-pix_fmt",
    //     "rgb24",
    //     "-s",
    //     &format!("{}x{}", width, height),
    //     "-r",
    //     &framerate.to_string(),
    //     "-i",
    //     "pipe:0", // read from stdin
    //     "-c:v",
    //     "libx264",
    //     "-preset",
    //     "veryslow",
    //     "-crf",
    //     "23",
    //     "-pix_fmt",
    //     "yuv420p",
    //     "-f",
    //     "hls",
    //     "-hls_time",
    //     "10",
    //     "-hls_list_size",
    //     "0",
    //     &format!("{}/index.m3u8", output_dir),
    // ]
    // Start ffmpeg process
    let mut ffmpeg = Command::new("ffmpeg")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();

    let mut ffmpeg_stdin = ffmpeg.stdin.take().expect("Failed to open ffmpeg stdin");

    // Check if the platform is supported
    if !scap::is_supported() {
        info!("Platform not supported");
        return;
    }

    // Check if we have permission to capture screen
    // If we don't, request it.
    if !scap::has_permission() {
        info!("Requesting permission...");
        if !scap::request_permission() {
            info!("Permission denied for");
            return;
        }
    }

    // // Get recording targets
    // let targets = scap::get_all_targets();

    let targets = get_all_targets();

    for t in targets {
        match t {
            Target::Window(window) => {
                // info!("window :{:?}", window)
            }
            Target::Display(display) => {
                info!("display :{:?}", display)
            }
        }
    }
    // info!("targets {:?}", targets);
    // Create Options
    let options = Options {
        fps: framerate,
        show_cursor: true,
        show_highlight: true,
        excluded_targets: None,
        output_type: scap::frame::FrameType::RGB,
        output_resolution: scap::capturer::Resolution::_1080p,
        crop_area: None,
        //  Some(Area {
        //     origin: Point { x: 0.0, y: 0.0 },
        //     size: Size {
        //         width: 500.0,
        //         height: 500.0,
        //     },
        // }),
        ..Default::default()
    };

    // Create Recorder with options
    let mut recorder = Capturer::build(options).unwrap_or_else(|err| {
        info!("Problem with building Capturer: {err}");
        process::exit(1);
    });

    // Start Capture
    recorder.start_capture();

    // Capture 100 frames
    let mut start_time: u64 = 0;
    for i in 0..10 {
        let frame = recorder.get_next_frame().expect("Error");

        match frame {
            Frame::RGB(frame) => {
                if start_time == 0 {
                    start_time = frame.display_time;
                }

                info!(
                    "Recieved BGRA frame {} of width {} and height {} and time {}",
                    i,
                    frame.width,
                    frame.height,
                    frame.display_time - start_time
                );

                if frame.width == 0 || frame.height == 0 {
                    continue;
                }

                let img_buffer: ImageBuffer<Rgb<u8>, Vec<u8>> =
                    ImageBuffer::from_raw(frame.width as u32, frame.height as u32, frame.data)
                        .expect("Failed to create ImageBuffer from raw data");

                let dynamic_image = DynamicImage::ImageRgb8(img_buffer);

                // Resize to target resolution
                let resized: DynamicImage =
                    dynamic_image.resize_exact(width, height, FilterType::Triangle);

                let buffer = resized.as_bytes();

                ffmpeg_stdin.write_all(buffer).unwrap();
            }
            _ => {
                panic!();
            }
        }
    }

    // Stop Capture
    recorder.stop_capture();

    drop(ffmpeg_stdin); // Close stdin to let ffmpeg finalize output
    let status = ffmpeg.wait().unwrap();

    if status.success() {
        info!("✅ HLS output written to {}/index.m3u8", output_dir);
    } else {
        info!("❌ FFmpeg exited with error: {:?}", status);
    }
}
