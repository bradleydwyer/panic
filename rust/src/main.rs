use panic_demo::{
    data::Data,
    engine::{Engine, FRAME_RATE, SAMPLE_RATE},
};
use std::{
    env,
    ffi::OsString,
    fs,
    io::{BufWriter, Seek, Write},
    path::{Path, PathBuf},
};

const HELP: &str = "\
Panic — Future Crew (1992), native Rust port

Usage: panic-demo [options]
  --data PATH     Imported data directory (files/ and unpacked/) or research root
  --headless      Render without a window or audio device
  --out DIR       Save output: headless writes panic.wav, timeline.csv and a
                  snapshot every 10 seconds; desktop saves each scene's first
                  frame and desktop-run.json
  --seconds N     Stop after N seconds (counted after --seek)
  --seek N        Fast-forward N seconds before playback
  --mute          Silence the audio but keep music-driven timing
  --fallback      Use the original no-music scene schedules
  --fullscreen    Start fullscreen

Without --data, PANIC_DATA is used, then data/ in the current directory or
next to the executable, then the current directory.

Desktop controls: Escape quits, Space pauses, F toggles fullscreen,
Right/Left seek 10 seconds, Up/Down seek 60 seconds, Shift+Right/Left
jump to the next scene or back to the start of this or the previous one.
Use --release for smooth playback.";

/// Longest accepted --seek or --seconds value. The demo runs about 250 seconds.
const MAX_SECONDS: f64 = 3600.0;

#[derive(Debug, Default, PartialEq)]
struct Options {
    data: Option<PathBuf>,
    headless: bool,
    out: Option<PathBuf>,
    seconds: Option<f64>,
    seek: f64,
    mute: bool,
    fallback: bool,
    fullscreen: bool,
    help: bool,
}

fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Options, String> {
    fn value(
        args: &mut impl Iterator<Item = OsString>,
        flag: &str,
        what: &str,
    ) -> Result<OsString, String> {
        args.next().ok_or(format!("{flag} requires {what}"))
    }
    fn time(args: &mut impl Iterator<Item = OsString>, flag: &str) -> Result<f64, String> {
        let text = value(args, flag, "a number of seconds")?;
        text.to_str()
            .and_then(|t| t.parse::<f64>().ok())
            .filter(|t| t.is_finite() && (0.0..=MAX_SECONDS).contains(t))
            .ok_or(format!(
                "{flag} must be a number of seconds from 0 to {MAX_SECONDS}, not {}",
                text.to_string_lossy()
            ))
    }
    let mut o = Options::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let Some(flag) = arg.to_str() else {
            return Err(format!(
                "Unknown argument {}; use --help",
                arg.to_string_lossy()
            ));
        };
        match flag {
            "--data" => o.data = Some(PathBuf::from(value(&mut args, flag, "a path")?)),
            "--out" => o.out = Some(PathBuf::from(value(&mut args, flag, "a directory")?)),
            "--seconds" => {
                let seconds = time(&mut args, flag)?;
                if seconds == 0.0 {
                    return Err("--seconds must be greater than 0".into());
                }
                o.seconds = Some(seconds);
            }
            "--seek" => o.seek = time(&mut args, flag)?,
            "--headless" => o.headless = true,
            "--mute" => o.mute = true,
            "--fallback" => o.fallback = true,
            "--fullscreen" => o.fullscreen = true,
            "-h" | "--help" => o.help = true,
            _ => return Err(format!("Unknown argument {flag}; use --help")),
        }
    }
    Ok(o)
}

/// Picks the data directory: --data, then PANIC_DATA, then a data/ directory
/// that looks complete, then the current directory (e.g. the research root).
fn data_root(o: &Options) -> Result<PathBuf, String> {
    if let Some(path) = &o.data {
        return Ok(path.clone());
    }
    if let Some(path) = env::var_os("PANIC_DATA").filter(|p| !p.is_empty()) {
        return Ok(PathBuf::from(path));
    }
    let executable = env::current_exe().map_err(|e| e.to_string())?;
    let parent = executable
        .parent()
        .ok_or("Executable has no parent directory")?;
    Ok([
        PathBuf::from("data"),
        parent.join("data"),
        parent.join("../Resources/data"),
    ]
    .into_iter()
    .find(|candidate| candidate.join("files/P.P").is_file())
    .unwrap_or_else(|| PathBuf::from(".")))
}

/// Maps an I/O error to a message that names the file involved.
fn io(path: &Path) -> impl FnOnce(std::io::Error) -> String + '_ {
    move |e| format!("{}: {e}", path.display())
}

/// 16 kHz stereo WAV writer. Writes to `<name>.partial` and renames it once
/// the header is complete, so an interrupted run never leaves a bad WAV.
struct Wav {
    file: BufWriter<fs::File>,
    partial: PathBuf,
    path: PathBuf,
    bytes: u32,
}
impl Wav {
    fn new(path: &Path) -> Result<Self, String> {
        let partial = partial(path);
        let mut w = Self {
            file: BufWriter::new(fs::File::create(&partial).map_err(io(&partial))?),
            partial,
            path: path.to_path_buf(),
            bytes: 0,
        };
        w.file.write_all(&[0; 44]).map_err(io(&w.partial))?;
        Ok(w)
    }
    fn write(&mut self, pcm: &[i16]) -> Result<(), String> {
        for &s in pcm {
            self.file
                .write_all(&s.to_le_bytes())
                .map_err(io(&self.partial))?;
        }
        self.bytes = pcm
            .len()
            .checked_mul(2)
            .and_then(|n| u32::try_from(n).ok())
            .and_then(|n| self.bytes.checked_add(n))
            .filter(|&n| n <= u32::MAX - 36)
            .ok_or("WAV output exceeds 4 GB")?;
        Ok(())
    }
    fn finish(mut self) -> Result<(), String> {
        self.file
            .rewind()
            .and_then(|_| self.file.write_all(&wav_header(self.bytes)))
            .and_then(|_| self.file.flush())
            .map_err(io(&self.partial))?;
        drop(self.file);
        fs::rename(&self.partial, &self.path).map_err(io(&self.path))
    }
}
fn wav_header(bytes: u32) -> Vec<u8> {
    let mut h = Vec::from(*b"RIFF");
    h.extend((bytes + 36).to_le_bytes());
    h.extend(b"WAVEfmt ");
    h.extend(16u32.to_le_bytes());
    h.extend(1u16.to_le_bytes());
    h.extend(2u16.to_le_bytes());
    h.extend((SAMPLE_RATE as u32).to_le_bytes());
    h.extend((SAMPLE_RATE as u32 * 4).to_le_bytes());
    h.extend(4u16.to_le_bytes());
    h.extend(16u16.to_le_bytes());
    h.extend(b"data");
    h.extend(bytes.to_le_bytes());
    h
}
fn partial(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".partial");
    PathBuf::from(name)
}
fn ppm(path: &Path, engine: &Engine) -> Result<(), String> {
    let v = &engine.video;
    let mut f = BufWriter::new(fs::File::create(path).map_err(io(path))?);
    write!(f, "P6\n{} {}\n255\n", v.width, v.height)
        .and_then(|_| f.write_all(&v.rgb()))
        .and_then(|_| f.flush())
        .map_err(io(path))
}
fn headless(engine: Engine, o: &Options) -> Result<(), String> {
    headless_run(engine, o).map_err(|e| match &o.out {
        Some(dir) => format!(
            "{e}\nOutput in {} is incomplete; unfinished files end in .partial",
            dir.display()
        ),
        None => e,
    })
}
fn headless_run(mut engine: Engine, o: &Options) -> Result<(), String> {
    let end = o.seconds.map(|s| {
        engine
            .frames
            .saturating_add((s * FRAME_RATE as f64).ceil() as u64)
    });
    let mut wav = None;
    let mut timeline = None;
    if let Some(dir) = &o.out {
        fs::create_dir_all(dir).map_err(io(dir))?;
        wav = Some(Wav::new(&dir.join("panic.wav"))?);
        let path = partial(&dir.join("timeline.csv"));
        let mut csv = BufWriter::new(fs::File::create(&path).map_err(io(&path))?);
        writeln!(
            csv,
            "frame,seconds,part,music_order,music_row,music_tick,width,height"
        )
        .map_err(io(&path))?;
        timeline = Some((csv, path));
    }
    let start = std::time::Instant::now();
    let mut part = 0;
    while !engine.finished && end.is_none_or(|e| engine.frames < e) {
        let pcm = engine.step()?;
        if let Some(w) = &mut wav {
            w.write(&pcm)?;
        }
        let changed = engine.part() != part;
        if changed {
            part = engine.part();
            println!(
                "{:8.3}s  part {}  music {:04x}",
                engine.seconds(),
                part,
                engine.music.position()
            );
        }
        if let Some((t, path)) = &mut timeline {
            writeln!(
                t,
                "{},{:.6},{},{},{},{},{},{}",
                engine.frames,
                engine.seconds(),
                engine.part(),
                engine.music.position() >> 8,
                engine.music.position() & 255,
                engine.music.tick(),
                engine.video.width,
                engine.video.height
            )
            .map_err(io(path))?;
        }
        if (changed || engine.frames.is_multiple_of(FRAME_RATE * 10) || engine.finished)
            && let Some(dir) = &o.out
        {
            ppm(
                &dir.join(format!(
                    "frame-{:06}-part{}.ppm",
                    engine.frames,
                    engine.part()
                )),
                &engine,
            )?;
        }
    }
    if let Some(w) = wav {
        w.finish()?;
    }
    if let (Some((mut t, path)), Some(dir)) = (timeline, &o.out) {
        t.flush().map_err(io(&path))?;
        drop(t);
        let csv = dir.join("timeline.csv");
        fs::rename(&path, &csv).map_err(io(&csv))?;
    }
    if let Some(dir) = &o.out {
        ppm(&dir.join("last.ppm"), &engine)?;
    }
    println!(
        "{} at {:.3}s ({} frames), music {:04x}/tick {}; rendered in {:.2}s",
        if engine.finished {
            "Completed"
        } else {
            "Stopped"
        },
        engine.seconds(),
        engine.frames,
        engine.music.position(),
        engine.music.tick(),
        start.elapsed().as_secs_f64()
    );
    Ok(())
}

#[cfg(feature = "desktop")]
fn desktop(mut engine: Engine, o: &Options) -> Result<(), String> {
    use sdl2::{
        audio::AudioSpecDesired,
        event::Event,
        keyboard::{Keycode, Mod},
        pixels::PixelFormatEnum,
        rect::Rect,
        render::Canvas,
        video::FullscreenType,
        video::Window,
    };
    use std::{
        collections::VecDeque,
        thread::sleep,
        time::{Duration, Instant},
    };
    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let audio = sdl.audio()?;
    let window = || {
        let mut builder = video.window("Panic — Future Crew / Rust", 960, 720);
        builder.position_centered().resizable().allow_highdpi();
        builder.build().map_err(|e| e.to_string())
    };
    // Prefer a GPU renderer; fall back to software (VMs, remote desktops).
    let mut canvas: Canvas<Window> = match window()?
        .into_canvas()
        .accelerated()
        .present_vsync()
        .build()
    {
        Ok(canvas) => canvas,
        Err(accelerated) => window()?
            .into_canvas()
            .software()
            .build()
            .map_err(|software| {
                format!("No renderer available (accelerated: {accelerated}; software: {software})")
            })?,
    };
    if o.fullscreen {
        canvas
            .window_mut()
            .set_fullscreen(FullscreenType::Desktop)?;
    }
    let creator = canvas.texture_creator();
    let mut texture_size = (engine.video.width, engine.video.height);
    let mut texture = creator
        .create_texture_streaming(
            PixelFormatEnum::RGB24,
            texture_size.0 as u32,
            texture_size.1 as u32,
        )
        .map_err(|e| e.to_string())?;
    let device = audio.open_queue::<i16, _>(
        None,
        &AudioSpecDesired {
            freq: Some(SAMPLE_RATE as i32),
            channels: Some(2),
            samples: Some(512),
        },
    )?;
    if device.spec().freq != SAMPLE_RATE as i32 || device.spec().channels != 2 {
        return Err("Audio device did not accept 16 kHz stereo".into());
    }
    // Samples handed to SDL are still in its device buffer before they are
    // heard, so pick video frames that far behind the queue. Keep enough
    // audio queued ahead to ride out a slow frame or a scene change.
    let latency = u64::from(device.spec().samples);
    let lead = (latency * 3).max(SAMPLE_RATE / 10);
    let mut events = sdl.event_pump()?;
    let mut origin = engine.audio_frames();
    let end = o
        .seconds
        .map(|s| origin.saturating_add((s * SAMPLE_RATE as f64).ceil() as u64));
    let mut submitted = 0u64;
    let mut paused = false;
    let mut full = o.fullscreen;
    let mut started = false;
    let mut queue = VecDeque::new();
    let mut shown = (
        engine.video.rgb(),
        engine.video.width,
        engine.video.height,
        engine.part(),
    );
    let mut shown_frame = engine.frames;
    let mut previous_part = 0;
    let wall_start = Instant::now();
    let mut presentations = 0u64;
    if let Some(dir) = &o.out {
        fs::create_dir_all(dir).map_err(io(dir))?;
    }
    'running: loop {
        let iteration = Instant::now();
        for event in events.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                Event::KeyDown {
                    keycode: Some(Keycode::Space),
                    repeat: false,
                    ..
                } => {
                    paused = !paused;
                    if paused {
                        device.pause();
                    } else {
                        device.resume();
                    }
                }
                Event::KeyDown {
                    keycode:
                        Some(key @ (Keycode::Right | Keycode::Left | Keycode::Up | Keycode::Down)),
                    keymod,
                    ..
                } => {
                    let shift = keymod.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD);
                    let forward = matches!(key, Keycode::Right | Keycode::Up);
                    if shift && matches!(key, Keycode::Right | Keycode::Left) {
                        // Next scene, or back to the start of this one; if
                        // this one started under 2 seconds ago, the one before.
                        let scene = engine.scene_at(shown_frame);
                        let index = if forward {
                            scene + 1
                        } else if shown_frame - engine.scene_start(scene).unwrap_or(0)
                            < 2 * FRAME_RATE
                        {
                            scene.saturating_sub(1)
                        } else {
                            scene
                        };
                        engine.seek_scene(index)?;
                    } else {
                        let step = FRAME_RATE
                            * if matches!(key, Keycode::Up | Keycode::Down) {
                                60
                            } else {
                                10
                            };
                        engine.seek(if forward {
                            shown_frame.saturating_add(step)
                        } else {
                            shown_frame.saturating_sub(step)
                        })?;
                    }
                    // Drop audio and frames rendered for the old position.
                    device.clear();
                    queue.clear();
                    origin = engine.audio_frames();
                    submitted = 0;
                    shown_frame = engine.frames;
                    shown = (
                        engine.video.rgb(),
                        engine.video.width,
                        engine.video.height,
                        engine.part(),
                    );
                    let seconds = engine.seconds();
                    println!(
                        "seek {}:{:06.3}  part {}",
                        (seconds / 60.0) as u64,
                        seconds % 60.0,
                        engine.part()
                    );
                }
                Event::KeyDown {
                    keycode: Some(Keycode::F),
                    repeat: false,
                    ..
                } => {
                    full = !full;
                    canvas.window_mut().set_fullscreen(if full {
                        FullscreenType::Desktop
                    } else {
                        FullscreenType::Off
                    })?;
                }
                _ => {}
            }
        }
        let played = submitted.saturating_sub(device.size() as u64 / 4);
        if !paused {
            while !engine.finished
                && engine.audio_frames() - origin < played + lead
                && end.is_none_or(|e| engine.audio_frames() < e)
            {
                let start = engine.audio_frames() - origin;
                let mut pcm = engine.step()?;
                let count = pcm.len() as u64 / 2;
                queue.push_back((
                    start,
                    engine.frames,
                    engine.video.rgb(),
                    engine.video.width,
                    engine.video.height,
                    engine.part(),
                ));
                if o.mute {
                    pcm.fill(0);
                }
                device.queue_audio(&pcm)?;
                submitted += count;
            }
            if !started {
                device.resume();
                started = true;
            }
            let audible = played.saturating_sub(latency);
            while queue.front().is_some_and(|f| f.0 <= audible) {
                if let Some((_, frame, rgb, w, h, p)) = queue.pop_front() {
                    shown = (rgb, w, h, p);
                    shown_frame = frame;
                }
            }
            if (engine.finished || end.is_some_and(|e| engine.audio_frames() >= e))
                && played >= submitted
            {
                break;
            }
        }
        let (rgb, w, h, part) = &shown;
        let changed = *part != previous_part;
        if changed {
            println!("part {}", part);
            previous_part = *part;
        }
        if texture_size != (*w, *h) {
            texture_size = (*w, *h);
            texture = creator
                .create_texture_streaming(PixelFormatEnum::RGB24, *w as u32, *h as u32)
                .map_err(|e| e.to_string())?;
        }
        texture
            .update(None, rgb, *w * 3)
            .map_err(|e| e.to_string())?;
        let (cw, ch) = canvas.output_size()?;
        let (dw, dh) = if cw * 3 > ch * 4 {
            (ch * 4 / 3, ch)
        } else {
            (cw, cw * 3 / 4)
        };
        canvas.set_draw_color(sdl2::pixels::Color::BLACK);
        canvas.clear();
        canvas.copy(
            &texture,
            None,
            Rect::new(((cw - dw) / 2) as i32, ((ch - dh) / 2) as i32, dw, dh),
        )?;
        if changed && let Some(dir) = &o.out {
            let readback = canvas.read_pixels(None, PixelFormatEnum::RGB24)?;
            let path = dir.join(format!("desktop-part{part}.ppm"));
            let mut file = BufWriter::new(fs::File::create(&path).map_err(io(&path))?);
            write!(file, "P6\n{cw} {ch}\n255\n")
                .and_then(|_| file.write_all(&readback))
                .and_then(|_| file.flush())
                .map_err(io(&path))?;
        }
        canvas.present();
        presentations += 1;
        // Without working vsync (or while paused) present() returns at once;
        // don't spin a CPU core.
        if iteration.elapsed() < Duration::from_millis(2) {
            sleep(Duration::from_millis(4));
        }
    }
    device.pause();
    if let Some(dir) = &o.out {
        let path = dir.join("desktop-run.json");
        fs::write(&path, format!(
            "{{\n  \"renderer\": \"{}\",\n  \"audio_driver\": \"{}\",\n  \"sample_rate\": {},\n  \"channels\": {},\n  \"device_buffer_frames\": {},\n  \"queue_lead_frames\": {},\n  \"queued_frames\": {},\n  \"remaining_frames\": {},\n  \"presentations\": {},\n  \"elapsed_seconds\": {:.6},\n  \"demo_seconds\": {:.6},\n  \"part\": {},\n  \"complete\": {}\n}}\n",
            canvas.info().name,
            audio.current_audio_driver(),
            device.spec().freq,
            device.spec().channels,
            latency,
            lead,
            submitted,
            device.size() / 4,
            presentations,
            wall_start.elapsed().as_secs_f64(),
            engine.seconds(),
            engine.part(),
            engine.finished
        ))
        .map_err(io(&path))?;
    }
    Ok(())
}

fn run() -> Result<(), String> {
    let o = parse(env::args_os().skip(1))?;
    if o.help {
        println!("{HELP}");
        return Ok(());
    }
    if !o.headless && !cfg!(feature = "desktop") {
        return Err(
            "Desktop support is disabled in this build; use --headless or rebuild with default features"
                .into(),
        );
    }
    let mut engine = Engine::new(Data::new(data_root(&o)?)?, o.fallback)?;
    engine.seek((o.seek * FRAME_RATE as f64).floor() as u64)?;
    if o.headless {
        return headless(engine, &o);
    }
    #[cfg(feature = "desktop")]
    {
        desktop(engine, &o)
    }
    #[cfg(not(feature = "desktop"))]
    {
        unreachable!("checked above")
    }
}
fn main() {
    if let Err(e) = run() {
        eprintln!("panic-demo: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Result<Options, String> {
        parse(list.iter().map(OsString::from))
    }

    #[test]
    fn parses_every_option() {
        let o = args(&[
            "--data",
            "d",
            "--headless",
            "--out",
            "o",
            "--seconds",
            "10",
            "--seek",
            "180",
            "--mute",
            "--fallback",
            "--fullscreen",
        ])
        .unwrap();
        assert_eq!(
            o,
            Options {
                data: Some("d".into()),
                headless: true,
                out: Some("o".into()),
                seconds: Some(10.0),
                seek: 180.0,
                mute: true,
                fallback: true,
                fullscreen: true,
                help: false,
            }
        );
        assert!(args(&["-h"]).unwrap().help);
        // An explicit "." is kept rather than treated as unset.
        assert_eq!(args(&["--data", "."]).unwrap().data, Some(".".into()));
    }

    #[test]
    fn rejects_bad_times_and_arguments() {
        for bad in [
            &["--seconds", "0"][..],
            &["--seconds", "-1"],
            &["--seconds", "1e300"],
            &["--seconds", "NaN"],
            &["--seek", "inf"],
            &["--seek", "-5"],
            &["--seek", "ten"],
            &["--seek"],
            &["--data"],
            &["--bogus"],
        ] {
            assert!(args(bad).is_err(), "{bad:?}");
        }
        assert_eq!(args(&["--seek", "0"]).unwrap().seek, 0.0);
        assert_eq!(args(&["--seconds", "3600"]).unwrap().seconds, Some(3600.0));
    }

    #[cfg(unix)]
    #[test]
    fn accepts_non_utf8_paths() {
        use std::os::unix::ffi::OsStringExt;
        let path = OsString::from_vec(b"/tmp/\xff".to_vec());
        let o = parse([OsString::from("--data"), path.clone()]).unwrap();
        assert_eq!(o.data, Some(PathBuf::from(path)));
        let flag = OsString::from_vec(b"--\xff".to_vec());
        assert!(parse([flag]).is_err());
    }

    #[test]
    fn wav_header_describes_16_khz_stereo_pcm() {
        let h = wav_header(64000);
        assert_eq!(h.len(), 44);
        assert_eq!(&h[..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(h[4..8].try_into().unwrap()), 64036);
        assert_eq!(&h[8..16], b"WAVEfmt ");
        assert_eq!(u16::from_le_bytes([h[22], h[23]]), 2);
        assert_eq!(u32::from_le_bytes(h[24..28].try_into().unwrap()), 16000);
        assert_eq!(u32::from_le_bytes(h[28..32].try_into().unwrap()), 64000);
        assert_eq!(u16::from_le_bytes([h[34], h[35]]), 16);
        assert_eq!(&h[36..40], b"data");
        assert_eq!(u32::from_le_bytes(h[40..44].try_into().unwrap()), 64000);
    }

    #[test]
    fn wav_is_renamed_only_when_finished() {
        let dir = env::temp_dir().join(format!("panic-wav-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("out.wav");
        let mut wav = Wav::new(&path).unwrap();
        wav.write(&[1, -1, 2, -2]).unwrap();
        assert!(!path.exists());
        assert!(partial(&path).exists());
        wav.finish().unwrap();
        let bytes = fs::read(&path).unwrap();
        assert!(!partial(&path).exists());
        assert_eq!(bytes.len(), 44 + 8);
        assert_eq!(&bytes[..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(bytes[40..44].try_into().unwrap()), 8);
        fs::remove_dir_all(&dir).unwrap();
    }
}
