//! Independent reconstruction of Panic's resident STMIK PCM engine.
//!
//! The equations and integer rules were recovered from the original resident
//! player, not taken from a contemporary tracker library. No original
//! instructions are executed.
//! Output is 16,000 stereo frames/s. A deterministic 512-byte refill budget
//! retains the original +0x41 chunk rounding and signed tick residue. Hardware
//! IRQ/DMA jitter is deliberately an input contract, not emulated here.
//! This is Panic's P.P playback profile, not a general S3M compatibility layer.
//! Samples are laid out in one deterministic arena (see `from_bytes`) so the
//! original kernel's small reads past a sample's guard bytes are reproducible.
use std::collections::VecDeque;

const RATE: u32 = 16_000;
const BYTE_RATE: u32 = RATE * 2;
const BASE: usize = 0x144b0;
const ROUTES: [(i32, i32); 8] = [
    (1, 0),
    (0, 1),
    (2, 0),
    (0, 2),
    (2, 1),
    (1, 2),
    (1, 1),
    (2, 2),
];
fn word(b: &[u8], p: usize) -> u16 {
    u16::from_le_bytes([b[p], b[p + 1]])
}
fn dword(b: &[u8], p: usize) -> u32 {
    u32::from_le_bytes(b[p..p + 4].try_into().unwrap())
}

// Preserve the recovered record's field widths and effect-memory aliasing.
// Offsets: 04 position, 08 end, 0c loop start, 12 current period, 14/16 step,
// 18 fraction, 1a volume, 1b route, 20 target, 22 C2 rate, 24 base period,
// 26 phase, 2a baseline volume, 2b/2c H/G memory, 32..36 row event,
// 37/38 remembered instrument/note, 39 shared effect memory, 3e cue expiry.
// These are ordinary player fields, not CPU registers or executable memory.
#[derive(Clone)]
struct Channel {
    r: [u8; 64],
    sample: Option<usize>,
}
impl Channel {
    fn new(n: usize) -> Self {
        let mut r = [0; 64];
        r[3] = n as u8;
        r[0x3a] = 101;
        Self { r, sample: None }
    }
    fn w(&self, p: usize) -> u16 {
        word(&self.r, p)
    }
    fn set(&mut self, p: usize, v: u16) {
        self.r[p..p + 2].copy_from_slice(&v.to_le_bytes());
    }
}
#[derive(Clone, Default)]
struct Sample {
    data: Vec<u8>,
    end: u16,
    loop_start: u16,
    c2: u16,
    volume: u8,
    valid: bool,
}
#[derive(Clone, Copy, Default)]
struct Event {
    channel: usize,
    note: Option<u8>,
    instrument: Option<u8>,
    volume: Option<u8>,
    effect: Option<(u8, u8)>,
}

pub struct Music {
    samples: Vec<Sample>,
    patterns: Vec<Vec<Vec<Event>>>,
    orders: Vec<u8>,
    channels: [Channel; 16],
    periods: [u16; 16],
    octaves: [u8; 16],
    waves: [[i16; 64]; 3],
    gains: [u8; 64],
    volume_tables: [[u8; 256]; 48],
    clip: Vec<u8>,
    channel_map: [u8; 32],
    order: usize,
    row: u16,
    tick_index: u16,
    speed: u16,
    tempo: u16,
    global_volume: u16,
    flags: u16,
    seed: u16,
    row_delay: u8,
    loop_count: u8,
    loop_row: u16,
    jump: Option<(usize, u16)>,
    tick_remaining: i32,
    pending: VecDeque<i16>,
    ended: bool,
    cue_counter: u16,
    accum_a: Vec<i32>,
    accum_b: Vec<i32>,
}

impl Music {
    pub fn new(data: &crate::data::Data) -> Result<Self, String> {
        let song = data.file("P.P")?;
        let memory = data.memory(0)?;
        Self::from_bytes(&song, &memory)
    }
    fn from_bytes(song: &[u8], memory: &[u8]) -> Result<Self, String> {
        if song.len() < 96 || &song[44..48] != b"SCRM" {
            return Err("P.P is not an S3M module".into());
        }
        if memory.len() < BASE + 0x5bb4 {
            return Err("Resident lookup tables are missing".into());
        }
        let ord = word(song, 32) as usize;
        let ins = word(song, 34) as usize;
        let pat = word(song, 36) as usize;
        if 96 + ord + 2 * (ins + pat) > song.len() {
            return Err("Truncated S3M directory".into());
        }
        let mut channel_map = [255; 32];
        channel_map.copy_from_slice(&song[64..96]);
        if channel_map.iter().any(|x| x & 128 == 0 && *x > 15) {
            return Err("Panic's port supports PCM channels, not OPL instruments".into());
        }
        let mut samples = Vec::new();
        for i in 0..ins {
            let h = word(song, 96 + ord + i * 2) as usize * 16;
            if h + 80 > song.len() {
                return Err(format!("Truncated sample header {i}"));
            }
            if song[h] != 1 {
                samples.push(Sample::default());
                continue;
            }
            if song[h + 30] != 0 || song[h + 31] & 6 != 0 {
                return Err("Packed/16-bit/stereo source samples are unsupported".into());
            }
            let length = dword(song, h + 16).min(64000) as usize;
            let source = word(song, h + 14) as usize * 16;
            if source + length > song.len() {
                return Err(format!("Truncated sample data {i}"));
            }
            let start = dword(song, h + 20).min(length as u32) as usize;
            let end = dword(song, h + 24).min(length as u32) as usize;
            let looping = song[h + 31] & 1 != 0 && start < end && length > 0;
            let mut bytes = vec![128; length + 512];
            bytes[..length].copy_from_slice(&song[source..source + length]);
            if looping {
                for k in 0..512 {
                    bytes[end + k] = bytes[start + k];
                }
            }
            samples.push(Sample {
                data: bytes,
                end: if looping { end as u16 } else { length as u16 },
                loop_start: if looping { start as u16 } else { 65535 },
                c2: dword(song, h + 32).min(65535) as u16,
                volume: (song[h + 28] as i8).clamp(0, 63) as u8,
                valid: true,
            });
        }
        // The original 32-frame unrolled kernel can overrun its 512-byte
        // source guard. Use the oracle's explicit conventional-memory layout:
        // guarded samples in instrument order, zero padding to 16 bytes.
        // This gives deterministic neighboring bytes without host out-of-bounds
        // reads; real DOS/EMS allocation can produce different brief artifacts.
        let mut arena = Vec::new();
        let mut offsets = Vec::new();
        for sample in &samples {
            offsets.push(arena.len());
            if !sample.valid {
                continue;
            }
            arena.extend_from_slice(&sample.data);
            arena.resize((arena.len() + 15) & !15, 0);
        }
        arena.resize(arena.len() + 1024, 0);
        for (sample, offset) in samples.iter_mut().zip(offsets) {
            if sample.valid {
                let end = offset + sample.data.len();
                sample.data.extend_from_slice(&arena[end..end + 512]);
            }
        }
        let mut patterns = Vec::new();
        for i in 0..pat {
            let p = word(song, 96 + ord + ins * 2 + i * 2) as usize * 16;
            let mut rows = vec![Vec::new(); 64];
            if p != 0 {
                if p + 2 > song.len() {
                    return Err("Truncated pattern".into());
                }
                let mut cursor = p + 2;
                for row in &mut rows {
                    loop {
                        let token = *song.get(cursor).ok_or("Truncated packed pattern")?;
                        cursor += 1;
                        if token == 0 {
                            break;
                        }
                        let map = channel_map[(token & 31) as usize];
                        let mut event = Event {
                            channel: map as usize,
                            ..Event::default()
                        };
                        if token & 32 != 0 {
                            event.note = Some(*song.get(cursor).ok_or("Truncated note")?);
                            event.instrument =
                                Some(*song.get(cursor + 1).ok_or("Truncated instrument")?);
                            cursor += 2;
                        }
                        if token & 64 != 0 {
                            event.volume = Some(*song.get(cursor).ok_or("Truncated volume")?);
                            cursor += 1;
                        }
                        if token & 128 != 0 {
                            event.effect = Some((
                                *song.get(cursor).ok_or("Truncated effect")?,
                                *song.get(cursor + 1).ok_or("Truncated parameter")?,
                            ));
                            cursor += 2;
                        }
                        if map & 128 == 0 {
                            row.push(event)
                        }
                    }
                }
            }
            patterns.push(rows);
        }
        let mut periods = [0; 16];
        for (i, p) in periods.iter_mut().enumerate() {
            *p = word(memory, BASE + 0x5a08 + i * 2)
        }
        let mut octaves = [0; 16];
        octaves.copy_from_slice(&memory[BASE + 0x5960..BASE + 0x5970]);
        let mut gains = [0; 64];
        gains.copy_from_slice(&memory[BASE + 0x59a8..BASE + 0x59e8]);
        let mut waves = [[0; 64]; 3];
        for (j, off) in [0x5a34, 0x5b34, 0x5ab4].iter().enumerate() {
            for (i, value) in waves[j].iter_mut().enumerate() {
                *value = word(memory, BASE + off + i * 2) as i16
            }
        }
        let flags = word(song, 38);
        if flags & 0x10 != 0 {
            return Err("Legacy Amiga pitch-limit mode is not part of Panic's P.P profile".into());
        }
        let mut result = Self {
            samples,
            patterns,
            orders: song[96..96 + ord].to_vec(),
            channels: std::array::from_fn(Channel::new),
            periods,
            octaves,
            waves,
            gains,
            volume_tables: [[0; 256]; 48],
            clip: Vec::new(),
            channel_map,
            order: 0,
            row: 0,
            tick_index: 0,
            speed: song[49].max(1) as u16,
            tempo: song[50].max(33) as u16,
            global_volume: song[48].min(64) as u16,
            flags,
            seed: 0,
            row_delay: 0,
            loop_count: 0,
            loop_row: 0,
            jump: None,
            tick_remaining: 0,
            pending: VecDeque::new(),
            ended: false,
            cue_counter: 0,
            accum_a: vec![0; 2048],
            accum_b: vec![0; 2048],
        };
        let gain = ((song[51] & 127) as u16 * 11 / 8)
            .saturating_sub(2)
            .min(127);
        let width = 32768 / gain.max(16) as usize;
        let zero = ((2048 - width) / 2) & !1;
        let step = 65536 / width;
        result.clip = vec![0; zero];
        for i in 0..width {
            result.clip.push(((i * step) >> 8) as u8)
        }
        result.clip.resize(2048, 255);
        for row in 0..48 {
            let gain = result.gains[row * 348 / 256] as u16;
            for value in 0..256 {
                result.volume_tables[row][value] = (value as u16 * gain / 256) as u8
            }
        }
        if result.orders.first().copied().unwrap_or(255) == 255 {
            result.ended = true
        }
        Ok(result)
    }
    /// Current generated-audio order:row, including the original pre-decode
    /// row advance. Like a DMA ring, the pending refill may lead consumed audio.
    pub fn position(&self) -> u16 {
        ((self.order as u16) << 8) | self.row
    }
    pub fn tick(&self) -> u16 {
        self.tick_index
    }
    /// The first order pass has wrapped. Pending final-refill PCM is still
    /// returned by render; subsequent output is digital silence.
    pub fn ended(&self) -> bool {
        self.ended
    }
    /// Native unsigned 8-bit SB output is expanded exactly to signed 16-bit PCM.
    pub fn render(&mut self, frames: usize) -> Vec<i16> {
        let mut out = Vec::with_capacity(frames * 2);
        while out.len() < frames * 2 {
            if self.pending.is_empty() {
                if self.ended {
                    out.resize(frames * 2, 0);
                    break;
                }
                self.refill();
            }
            let take = (frames * 2 - out.len()).min(self.pending.len());
            out.extend(self.pending.drain(..take));
        }
        out
    }
    /// Part1 invokes INT60/AX10 on these zero-based animation frames.
    pub fn intro_cue(&mut self, frame: usize) {
        if ![58, 71, 80, 91, 103, 113, 140, 152, 165, 180].contains(&frame) {
            return;
        }
        let counter = self.cue_counter.min(9);
        let logical = 12 + ((counter & 1) * 2) as usize;
        for (n, vol) in [
            (logical, counter * 60 / 9),
            (logical + 1, (9 - counter) * 60 / 9),
        ] {
            let ch = self.channel_map[n];
            if ch & 128 != 0 {
                continue;
            }
            let c = &mut self.channels[ch as usize];
            c.r[0x32..0x37].copy_from_slice(&[0x56, 47, vol as u8, 0, 0]);
            c.r[0x3e] = self.speed as u8;
            c.r[0x3a] = 100;
            self.note(ch as usize);
        }
        self.cue_counter += 1;
    }
    fn refill(&mut self) {
        let mut work = 512i32;
        while work > 0 {
            let bytes = (work.min(self.tick_remaining) + 65) & !63;
            work -= bytes;
            self.tick_remaining -= bytes;
            if bytes >= 64 {
                let pcm = self.mix(bytes as usize / 2);
                self.pending.extend(pcm)
            }
            if self.tick_remaining <= 0 {
                self.step_tick();
                self.tick_remaining += (BYTE_RATE * 5 / (self.tempo as u32 * 2)) as i32;
            }
        }
    }
    fn note_period(&self, note: u8, c2: u16) -> u16 {
        if note == 254 || c2 == 0 {
            return 0;
        }
        let p =
            self.periods[(note & 15) as usize] >> (self.octaves[(note >> 4) as usize] & 31).min(15);
        ((p as u32 * 8363 / c2 as u32).min(32767)) as u16
    }
    fn update_step(c: &mut Channel) {
        let p = c.w(0x12).clamp(64, 32767) as u32;
        let f = 0xda7600u32 / p;
        c.set(0x1c, f as u16);
        c.set(0x1e, (f >> 16) as u16);
        c.set(0x14, ((f % RATE) * 65536 / RATE) as u16);
        c.set(0x16, (f / RATE) as u16);
    }
    fn note(&mut self, ch: usize) {
        let c = &mut self.channels[ch];
        c.r[1] = 1;
        if c.r[0x35] == 19 && c.r[0x36] & 240 == 208 {
            return;
        }
        self.note_now(ch)
    }
    fn note_now(&mut self, ch: usize) {
        let c = &mut self.channels[ch];
        if c.r[0x33] > 101 {
            c.r[0x33] = 0
        }
        if c.r[0x34] != 255 {
            c.r[0x34] = c.r[0x34].min(63)
        }
        if c.r[0x33] != 0 {
            let index = c.r[0x33] as usize - 1;
            c.r[0x37] = c.r[0x33];
            if let Some(s) = self.samples.get(index).filter(|s| s.valid) {
                c.sample = Some(index);
                c.set(0x22, s.c2);
                c.r[0x1a] = s.volume;
                c.r[0x2a] = s.volume;
                if c.r[0x35] != 7 {
                    c.set(4, 0)
                }
                c.set(8, s.end);
                c.set(12, s.loop_start);
            } else {
                c.r[0x37] = 0
            }
        }
        if c.r[0x37] == 0 {
            return;
        }
        let note = c.r[0x32];
        if note != 255 {
            if note == 254 {
                c.set(4, 1);
                c.set(0x18, 0);
                c.r[0x1a] = 0;
                c.set(8, 0);
                c.set(12, 65535);
                c.set(0x24, 65535);
                c.set(0x20, 65535);
            } else {
                if c.r[0x35] != 7 {
                    c.r[2] = 255;
                    c.set(4, 0);
                    c.set(0x18, 0)
                }
                c.r[0x38] = note;
                let p = self.periods[(note & 15) as usize]
                    >> (self.octaves[(note >> 4) as usize] & 31).min(15);
                let period = (p as u32 * 8363 / c.w(0x22).max(1) as u32).min(32767) as u16;
                if c.r[0x35] != 7 {
                    c.set(0x12, period);
                    Self::update_step(c);
                    c.set(0x26, 0)
                }
                c.set(0x24, period);
                c.set(0x20, period);
            }
        }
        if c.r[0x34] != 255 {
            c.r[0x1a] = c.r[0x34];
            c.r[0x2a] = c.r[0x34]
        }
        if c.r[2] == 255 {
            c.r[2] = c.r[0x1a] + 16
        }
    }
    fn step_tick(&mut self) {
        self.seed = (((0xcdefu32 * self.seed as u32) >> 16) as u16).wrapping_add(0x1727);
        if self.tick_index == 0 {
            if self.row_delay != 0 {
                self.row = self.row.wrapping_sub(1);
                self.row_delay -= 1;
            } else {
                for c in &mut self.channels {
                    c.r[0x32..0x37].copy_from_slice(&[255, 0, 255, 0, 0]);
                }
                let pat = *self.orders.get(self.order).unwrap_or(&255) as usize;
                let events = self
                    .patterns
                    .get(pat)
                    .and_then(|p| p.get(self.row as usize))
                    .cloned()
                    .unwrap_or_default();
                for e in events {
                    let c = &mut self.channels[e.channel];
                    if let Some(n) = e.note {
                        c.r[0x32] = n;
                        if n != 255 {
                            c.r[0x38] = n
                        }
                    }
                    if let Some(i) = e.instrument {
                        c.r[0x33] = i;
                        if i != 0 {
                            c.r[0x37] = i
                        }
                    }
                    if let Some(v) = e.volume {
                        c.r[0x34] = v
                    }
                    if let Some((f, p)) = e.effect {
                        c.r[0x35] = f;
                        c.r[0x36] = p
                    }
                    self.note(e.channel);
                }
            }
        }
        for ch in 0..16 {
            if self.channels[ch].r[1] == 0 {
                continue;
            }
            if self.tick_index == 0 {
                let c = &mut self.channels[ch];
                if c.r[0x3e] != 0 {
                    c.r[0x3e] -= 1;
                    if c.r[0x3e] == 0 {
                        c.r[0x32..0x37].copy_from_slice(&[255, 0, 255, 0, 0]);
                    }
                }
                if self.flags & 8 != 0
                    && c.r[0x1a] == 0
                    && c.r[0x34] == 255
                    && c.r[0x33] == 0
                    && c.r[0x32] == 255
                {
                    c.r[0x31] = c.r[0x31].wrapping_sub(1);
                    if c.r[0x31] == 0 {
                        c.r[0x1a] = 0;
                        c.set(8, 0);
                        c.set(4, 1);
                        c.r[1] = 0;
                        continue;
                    }
                } else {
                    c.r[0x31] = 3
                }
                let f = c.r[0x35];
                let p = c.r[0x36];
                if p != 0 {
                    c.r[0x39] = p
                }
                if f == 0 || f == 4 {
                    c.r[0x2e] = 0;
                    if c.w(0x24) != c.w(0x12) {
                        let p = c.w(0x24);
                        c.set(0x12, p);
                        Self::update_step(c)
                    }
                }
                if f != 0 && f != 4 {
                    if f != 9 {
                        c.r[0x2f] = 0;
                        c.r[0x2d] = 1
                    }
                    if ![8, 11, 18].contains(&f) {
                        c.r[0x26] |= 128
                    }
                }
            }
            self.effect(ch);
        }
        self.tick_index += 1;
        if self.tick_index >= self.speed {
            self.row = self.row.wrapping_add(1);
            self.tick_index = 0;
            if let Some((order, row)) = self.jump.take() {
                self.order = order;
                self.row = row;
                self.seed = 0;
            } else if self.row >= 64 {
                self.row = 0;
                self.order += 1;
                self.seed = 0;
            }
            if self.orders.get(self.order).copied().unwrap_or(255) == 255 {
                self.ended = true;
                self.order = 0;
                self.row = 0;
            }
        }
    }
    fn vibrato(&mut self, ch: usize, mut p: u8, combined: bool, tremolo: bool) {
        let c = &mut self.channels[ch];
        let memory = if tremolo { 0x39 } else { 0x2b };
        if !combined {
            if p == 0 {
                p = c.r[memory]
            }
            if p & 240 == 0 {
                p |= c.r[memory] & 240
            }
            c.r[memory] = p;
            c.r[0x36] = p;
        }
        if (!tremolo && c.w(0x24) == 0) || (tremolo && c.r[0x2a] == 0) {
            return;
        }
        let wave = if tremolo {
            c.r[0x3d] >> 5
        } else {
            c.r[0x3d] >> 1
        } & 7;
        let phase = if wave < 4 && c.w(0x26) & 128 != 0 {
            0
        } else {
            c.w(0x26) & 127
        };
        let value = self.waves[if wave & 3 == 3 {
            0
        } else {
            (wave & 3) as usize
        }][(phase / 2) as usize] as i32;
        let delta = (value * (p & 15) as i32)
            >> (if tremolo {
                7
            } else if self.flags & 1 != 0 {
                4
            } else {
                5
            });
        if tremolo {
            c.r[0x1a] = (c.r[0x2a] as i32 + delta).clamp(0, 63) as u8
        } else {
            c.set(0x12, (c.w(0x24) as i32 + delta) as u16);
            Self::update_step(c)
        }
        c.set(
            0x26,
            (phase + (p >> 4) as u16 * 2 + if wave & 3 == 3 { self.seed & 30 } else { 0 }) & 126,
        );
    }
    fn porta(&mut self, ch: usize, mut p: u8, combined: bool) {
        let c = &mut self.channels[ch];
        if c.w(0x24) == 0 {
            return;
        }
        if !combined {
            if p == 0 {
                p = c.r[0x2c]
            }
            c.r[0x2c] = p;
            c.r[0x36] = p
        }
        let current = c.w(0x12);
        let target = c.w(0x20);
        if current == target {
            return;
        }
        let new = if current < target {
            current.saturating_add(p as u16 * 4).min(target)
        } else {
            current.saturating_sub(p as u16 * 4).max(target)
        };
        c.set(0x24, new);
        c.set(0x12, new);
        Self::update_step(c);
    }
    fn effect(&mut self, ch: usize) {
        let first = self.tick_index == 0;
        let f = self.channels[ch].r[0x35];
        let mut p = self.channels[ch].r[0x36];
        if [4, 5, 6, 17, 18].contains(&f) && p == 0 {
            p = self.channels[ch].r[0x39];
            self.channels[ch].r[0x36] = p;
        }
        match f {
            1 if first => {
                if p != 0 {
                    self.speed = p as u16
                }
            }
            2 if first => self.jump = Some((p as usize, 0)),
            3 if first => self.jump = Some((self.order + 1, p as u16)),
            4 | 11 | 12 if f == 4 || !first => {
                let c = &mut self.channels[ch];
                if p == 0 {
                    p = c.r[0x39];
                    c.r[0x36] = p
                }
                c.r[0x3c] = 1;
                let (hi, lo) = (p >> 4, p & 15);
                let fine = (lo == 15 && hi != 0) || (hi == 15 && lo != 0);
                if fine && !first {
                    return;
                }
                let delta = if lo == 15 && hi != 0 {
                    hi as i16
                } else if lo != 0 {
                    -(lo as i16)
                } else {
                    hi as i16
                };
                c.r[0x1a] = (c.r[0x1a] as i16 + delta).clamp(0, 63) as u8;
                if f == 11 {
                    let p = c.r[0x2b];
                    self.vibrato(ch, p, true, false)
                }
                if f == 12 {
                    let p = self.channels[ch].r[0x2c];
                    self.porta(ch, p, true)
                }
            }
            5 | 6 => {
                let c = &mut self.channels[ch];
                if c.w(0x24) == 0 {
                    return;
                }
                let delta = if first {
                    if p > 0xf0 {
                        (p & 15) as u16 * 4
                    } else if p > 0xe0 {
                        (p & 15) as u16
                    } else {
                        0
                    }
                } else if p < 0xe0 {
                    p as u16 * 4
                } else {
                    0
                };
                let delta = if f == 6 {
                    0u16.wrapping_sub(delta)
                } else {
                    delta
                };
                c.set(0x12, c.w(0x12).wrapping_add(delta));
                c.set(0x24, c.w(0x24).wrapping_add(delta));
                Self::update_step(c);
            }
            7 if !first => self.porta(ch, p, false),
            8 if !first => self.vibrato(ch, p, false, false),
            9 => {
                let c = &mut self.channels[ch];
                if c.r[0x2f] != 0 {
                    c.r[0x2f] -= 1
                } else if c.r[0x2d] == 1 {
                    c.r[0x2d] = 0;
                    c.r[0x1a] = 0;
                    c.r[0x2f] = p & 15
                } else {
                    c.r[0x2d] = 1;
                    c.r[0x1a] = c.r[0x2a];
                    c.r[0x2f] = p >> 4
                }
            }
            10 => {
                let note = self.channels[ch].r[0x38];
                let mut semi = (note & 15)
                    + match self.tick_index % 3 {
                        1 => p >> 4,
                        2 => p & 15,
                        _ => 0,
                    };
                let mut octave = note >> 4;
                if semi >= 12 {
                    semi -= 12;
                    octave = octave.wrapping_add(1)
                }
                let period = self.note_period((octave << 4) | semi, self.channels[ch].w(0x22));
                let c = &mut self.channels[ch];
                c.set(0x12, period);
                Self::update_step(c)
            }
            15 if first => {
                let c = &mut self.channels[ch];
                let end = c.w(8);
                let mut pos = (p as u16) << 8;
                let lp = c.w(12);
                let mut guard = 0;
                while end != 0 && pos >= end && guard < 65536 {
                    pos = pos.wrapping_sub(end).wrapping_add(lp);
                    guard += 1
                }
                c.set(4, pos)
            }
            17 => {
                let c = &mut self.channels[ch];
                let interval = p & 15;
                if interval != 0 && interval <= c.r[0x2e] {
                    c.r[0x2e] = 0;
                    c.set(4, 0);
                    let code = (p >> 4) as usize;
                    let add = [0, -1, -2, -4, -8, -16, 0, 0, 0, 1, 2, 4, 8, 16, 0, 0][code];
                    let mul = [0, 0, 0, 0, 0, 0, 10, 8, 0, 0, 0, 0, 0, 0, 24, 32][code];
                    let v = if mul != 0 {
                        c.r[0x1a] as i16 * mul / 16
                    } else {
                        c.r[0x1a] as i16 + add
                    };
                    c.r[0x1a] = v.clamp(0, 63) as u8
                }
                c.r[0x2e] = c.r[0x2e].wrapping_add(1)
            }
            18 if !first => self.vibrato(ch, p, false, true),
            19 => {
                let x = p & 15;
                match (p >> 4, first) {
                    // S0x (filter) and S1x (glissando) are consumed like any
                    // other effect, but the original player never reads the
                    // state they set, so nothing is stored here.
                    (0, true) | (1, true) => {}
                    (3, true) => {
                        let c = &mut self.channels[ch];
                        c.r[0x3d] = (c.r[0x3d] & 240) | ((x << 1) & 15)
                    }
                    (4, true) => {
                        let c = &mut self.channels[ch];
                        c.r[0x3d] = (c.r[0x3d] & 15) | (x << 5)
                    }
                    (10, true) if x < 8 => self.channels[ch].r[0x1b] = x,
                    (11, true) => {
                        if x != 0 {
                            if self.loop_count == 0 {
                                self.loop_count = x;
                                self.loop_row = self.row.wrapping_sub(1)
                            }
                        } else if self.loop_count != 0 {
                            self.loop_count -= 1;
                            self.row = self.loop_row
                        }
                    }
                    (12, true) | (13, true) => self.channels[ch].r[0x2e] = x,
                    (12, false) | (13, false) => {
                        let c = &mut self.channels[ch];
                        if c.r[0x2e] != 0 {
                            c.r[0x2e] -= 1;
                            if c.r[0x2e] == 0 {
                                if p >> 4 == 12 {
                                    c.set(4, c.w(8))
                                } else {
                                    self.note_now(ch)
                                }
                            }
                        }
                    }
                    (14, true) if self.row_delay == 0 => self.row_delay = x,
                    _ => {}
                }
            }
            20 if first && p > 32 => self.tempo = p as u16,
            22 if !first && p <= 64 => self.global_volume = p as u16,
            // Zxx stores a sync byte for the host program in the original;
            // the player itself never reads it, so it has no effect here.
            26 => {}
            _ => {}
        }
    }
    fn mix(&mut self, frames: usize) -> Vec<i16> {
        let a = &mut self.accum_a;
        let b = &mut self.accum_b;
        let mut bias_a = 0;
        let mut bias_b = 0;
        for (number, c) in self.channels.iter_mut().enumerate() {
            let Some(index) = c.sample else { continue };
            if c.w(4) >= c.w(8) {
                continue;
            }
            let sample = &self.samples[index];
            let row = c.r[0x1a].min(63) as usize * (self.global_volume.min(64) * 3) as usize / 256;
            let table = &self.volume_tables[row];
            let mode = c.r[0x1b] & 7;
            let mode = if mode < 6 {
                mode ^ (number as u8 / 8)
            } else {
                mode
            };
            let (wa, wb) = ROUTES[mode as usize];
            let bias = table[128] as i32;
            bias_a += bias * wa;
            bias_b += bias * wb;
            let step = ((c.w(0x16) as u32) << 16) | c.w(0x14) as u32;
            let mut position = ((c.w(4) as u32) << 16) | c.w(0x18) as u32;
            // Original 91fc chooses one guard-safe subchunk near the end,
            // then normalizes after each whole subchunk, never per source byte.
            // The driver aligns subchunks to 16 frames, but each kernel unrolls
            // 32. Its overshoot advances source position and spills accumulator
            // words; conversion clears only consumed words. Keep that spill
            // between calls (it is audible in the original normal P.P path).
            let mut chunk = frames;
            let mut advance =
                ((step as u64 * chunk as u64 + (position & 65535) as u64) >> 16) as usize;
            if c.r[0x1a] != 0 {
                while (position >> 16) as usize + advance >= c.w(8) as usize && advance >= 512 {
                    advance /= 2;
                    chunk = (chunk / 2) & !15;
                }
                if chunk == 0 {
                    chunk = 16
                }
            }
            let mut done = 0;
            let mut remaining = frames;
            while remaining > 0 {
                let n = chunk.min(remaining);
                remaining -= n;
                let actual = (n + 31) & !31;
                for i in done..done + actual {
                    let value = sample
                        .data
                        .get((position >> 16) as usize)
                        .copied()
                        .unwrap_or(128);
                    let value = table[value as usize] as i32;
                    a[i] += value * wa;
                    b[i] += value * wb;
                    position = position.wrapping_add(step)
                }
                let mut pos = (position >> 16) as u16;
                if pos >= c.w(8) {
                    if c.w(12) == 65535 {
                        pos = c.w(8).wrapping_add(1)
                    } else {
                        let length = c.w(8).wrapping_sub(c.w(12));
                        if length == 0 {
                            pos = c.w(12)
                        } else {
                            while pos >= c.w(8) {
                                pos = pos.wrapping_sub(length)
                            }
                        }
                    }
                }
                position = ((pos as u32) << 16) | (position & 65535);
                done += actual;
                if pos as usize + advance <= c.w(8) as usize {
                    chunk = remaining;
                }
            }
            c.set(4, (position >> 16) as u16);
            c.set(0x18, position as u16);
        }
        let mut out = Vec::with_capacity(frames * 2);
        for i in 0..frames {
            for (value, bias) in [(b[i], bias_b), (a[i], bias_a)] {
                let index = 1024 + value - bias;
                let byte = self.clip[index.clamp(0, 2047) as usize];
                out.push((byte as i16 - 128) * 256)
            }
            a[i] = 0;
            b[i] = 0;
        }
        // The original's optional output filter (S0x) is never applied by the
        // player; P.P only disables it.
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    // A small invented S3M: one looping 8-bit unsigned sample and one packed
    // pattern whose first row holds `row0` (channel 0 maps to mixer channel 0).
    fn module(orders: &[u8], row0: &[u8]) -> Vec<u8> {
        fn align(s: &mut Vec<u8>) -> u16 {
            s.resize((s.len() + 15) & !15, 0);
            (s.len() / 16) as u16
        }
        let mut s = vec![0; 96];
        s[..4].copy_from_slice(b"test");
        s[28] = 0x1a;
        s[29] = 16;
        s[32..34].copy_from_slice(&(orders.len() as u16).to_le_bytes());
        s[34..36].copy_from_slice(&1u16.to_le_bytes());
        s[36..38].copy_from_slice(&1u16.to_le_bytes());
        s[44..48].copy_from_slice(b"SCRM");
        s[48] = 64;
        s[49] = 1;
        s[50] = 125;
        s[51] = 0xb0;
        s[64..96].fill(255);
        s[64] = 0;
        s[65] = 8;
        s.extend_from_slice(orders);
        let pointers = s.len();
        s.extend_from_slice(&[0; 4]);
        let header = align(&mut s);
        s[pointers..pointers + 2].copy_from_slice(&header.to_le_bytes());
        let h = s.len();
        s.resize(h + 80, 0);
        s[h] = 1;
        s[h + 16..h + 20].copy_from_slice(&32u32.to_le_bytes());
        s[h + 20..h + 24].copy_from_slice(&8u32.to_le_bytes());
        s[h + 24..h + 28].copy_from_slice(&32u32.to_le_bytes());
        s[h + 28] = 64;
        s[h + 31] = 1;
        s[h + 32..h + 36].copy_from_slice(&8363u32.to_le_bytes());
        s[h + 76..h + 80].copy_from_slice(b"SCRS");
        let data = align(&mut s);
        s[h + 14..h + 16].copy_from_slice(&data.to_le_bytes());
        s.extend((0..32u8).map(|i| 64 + i.min(32 - i) * 8));
        let pattern = align(&mut s);
        s[pointers + 2..pointers + 4].copy_from_slice(&pattern.to_le_bytes());
        let mut packed = row0.to_vec();
        packed.push(0);
        packed.extend_from_slice(&[0; 63]);
        s.extend_from_slice(&((packed.len() + 2) as u16).to_le_bytes());
        s.extend_from_slice(&packed);
        s
    }
    // Invented lookup tables: periods, octave shifts, gains and waveforms.
    fn tables() -> Vec<u8> {
        let mut m = vec![0; BASE + 0x5bb4];
        for i in 0..16 {
            let period = 7000 - i as u16 * 350;
            m[BASE + 0x5a08 + i * 2..][..2].copy_from_slice(&period.to_le_bytes());
            m[BASE + 0x5960 + i] = i as u8;
        }
        for i in 0..64 {
            m[BASE + 0x59a8 + i] = (i * 4) as u8;
        }
        for off in [0x5a34, 0x5b34, 0x5ab4] {
            for i in 0..64 {
                let v = (i as i16 - 32) * 8;
                m[BASE + off + i * 2..][..2].copy_from_slice(&v.to_le_bytes());
            }
        }
        m
    }
    // Note C-4 with instrument 1 and an effect on channel 0.
    fn row(effect: u8, parameter: u8) -> [u8; 5] {
        [32 | 128, 0x40, 1, effect, parameter]
    }
    fn synthetic(effect: u8, parameter: u8) -> Music {
        Music::from_bytes(&module(&[0, 255], &row(effect, parameter)), &tables()).unwrap()
    }
    fn error(song: &[u8]) -> String {
        match Music::from_bytes(song, &tables()) {
            Ok(_) => panic!("module was accepted"),
            Err(e) => e,
        }
    }
    #[test]
    fn synthetic_module_loads_and_plays_sound() {
        let mut m = synthetic(0, 0);
        assert!(!m.ended());
        let pcm = m.render(4000);
        assert_eq!(pcm.len(), 8000);
        assert!(pcm.iter().any(|&v| v != 0));
    }
    #[test]
    fn rejects_malformed_headers_and_directories() {
        let good = module(&[0, 255], &row(0, 0));
        let mut s = good.clone();
        s[44] = b'X';
        assert_eq!(error(&s), "P.P is not an S3M module");
        assert!(Music::from_bytes(&good, &tables()[..BASE + 0x5bb3]).is_err());
        let mut s = good.clone();
        s[32..34].copy_from_slice(&0xffffu16.to_le_bytes());
        assert_eq!(error(&s), "Truncated S3M directory");
        let mut s = good.clone();
        s[64 + 2] = 16;
        assert!(error(&s).contains("OPL"));
        let mut s = good.clone();
        s[38] = 0x10;
        assert!(error(&s).contains("Amiga"));
        let mut s = good.clone();
        s[38] = 8;
        assert!(Music::from_bytes(&s, &tables()).is_ok());
    }
    #[test]
    fn rejects_truncated_or_unsupported_samples_and_patterns() {
        let good = module(&[0, 255], &row(0, 0));
        let pointers = 96 + 2;
        let h = word(&good, pointers) as usize * 16;
        let mut s = good.clone();
        s[pointers..pointers + 2].copy_from_slice(&((good.len() / 16) as u16).to_le_bytes());
        assert_eq!(error(&s), "Truncated sample header 0");
        let mut s = good.clone();
        s[h + 16..h + 20].copy_from_slice(&60000u32.to_le_bytes());
        assert_eq!(error(&s), "Truncated sample data 0");
        for (offset, value) in [(30, 1), (31, 2), (31, 4)] {
            let mut s = good.clone();
            s[h + offset] = value;
            assert!(error(&s).contains("unsupported"), "byte {offset} = {value}");
        }
        let mut s = good.clone();
        s[h] = 2;
        assert!(Music::from_bytes(&s, &tables()).is_ok());
        let mut s = good.clone();
        s[pointers + 2..pointers + 4]
            .copy_from_slice(&((good.len() / 16 + 1) as u16).to_le_bytes());
        assert_eq!(error(&s), "Truncated pattern");
    }
    #[test]
    fn every_truncation_of_a_valid_module_is_an_error() {
        let good = module(&[0, 255], &row(20, 200));
        assert!(Music::from_bytes(&good, &tables()).is_ok());
        for n in 0..good.len() {
            assert!(
                Music::from_bytes(&good[..n], &tables()).is_err(),
                "prefix {n}"
            );
        }
    }
    #[test]
    fn empty_or_terminated_order_list_is_silent() {
        for orders in [&[][..], &[255, 0]] {
            let mut m = Music::from_bytes(&module(orders, &row(0, 0)), &tables()).unwrap();
            assert!(m.ended());
            assert_eq!(m.render(1000), vec![0; 2000]);
            assert_eq!(m.position(), 0);
        }
    }
    #[test]
    fn render_returns_two_samples_per_frame() {
        let mut m = synthetic(0, 0);
        for n in [0, 1, 2, 31, 32, 33, 255, 256, 257, 5000] {
            assert_eq!(m.render(n).len(), 2 * n);
        }
    }
    #[test]
    fn chunk_sizes_do_not_change_output() {
        let a = synthetic(0, 0).render(20000);
        let mut m = synthetic(0, 0);
        let mut b = Vec::new();
        for n in [1, 7, 333, 64, 0, 2000, 4095, 13500] {
            b.extend(m.render(n));
        }
        assert_eq!(a, b);
    }
    #[test]
    fn playback_ends_at_tempo_extremes() {
        // Txx below 33 is ignored; the header tempo of 125 stays in force.
        for (parameter, tempo) in [(33, 33), (255, 255), (32, 125)] {
            let mut m = synthetic(20, parameter);
            let mut frames = 0;
            while !m.ended() {
                m.render(1000);
                frames += 1000;
                assert!(frames < 200_000, "T{parameter} did not finish");
            }
            assert_eq!(m.tempo, tempo);
            // Drain the PCM of the final refill, after which only silence remains.
            m.render(2000);
            assert_eq!(m.render(1000), vec![0; 2000]);
        }
    }
    fn player() -> Music {
        let data = crate::test_support::data();
        Music::new(&data).unwrap()
    }
    #[test]
    #[ignore = "requires locally supplied PANIC_DATA and PANIC_FIXTURES; run with --ignored"]
    fn sequencer_matches_original_code_across_complete_order_pass() {
        let fixture = crate::test_support::fixture("tests/fixtures/music-sequencer.bin");
        let count = dword(&fixture, 0) as usize;
        let mut m = player();
        let mut tick = 0;
        for record in 0..count {
            let p = 4 + record * (16 + 16 * 64);
            let index = dword(&fixture, p);
            while tick <= index {
                m.step_tick();
                tick += 1;
            }
            let expected = (0..6)
                .map(|i| word(&fixture, p + 4 + i * 2))
                .collect::<Vec<_>>();
            assert_eq!(
                vec![
                    m.order as u16,
                    m.row,
                    m.tick_index,
                    m.speed,
                    m.tempo,
                    m.seed
                ],
                expected,
                "tick {index} globals"
            );
            for ch in 0..16 {
                let actual = &m.channels[ch].r;
                let expected = &fixture[p + 16 + ch * 64..p + 16 + (ch + 1) * 64];
                for offset in [
                    1, 3, 4, 5, 8, 9, 12, 13, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a,
                    0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27,
                    0x2a, 0x2b, 0x2c, 0x2d, 0x2e, 0x2f, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37,
                    0x38, 0x39, 0x3a, 0x3c, 0x3d, 0x3e,
                ] {
                    assert_eq!(
                        actual[offset], expected[offset],
                        "tick {index}, channel {ch}, field {offset:02x}"
                    );
                }
            }
        }
    }
    #[test]
    #[ignore = "requires locally supplied PANIC_DATA and PANIC_FIXTURES; run with --ignored"]
    fn complete_fixed_cadence_pcm_matches_original_bytes() {
        let f = crate::test_support::fixture("tests/fixtures/music-pcm.bin");
        let size = dword(&f, 0) as usize;
        let count = dword(&f, 4) as usize;
        let mut m = player();
        let pcm = m
            .render(size / 2)
            .into_iter()
            .map(|v| ((v as i32 / 256) + 128) as u8)
            .collect::<Vec<_>>();
        let mut cursor = 40;
        for _ in 0..count {
            let start = dword(&f, cursor) as usize;
            let n = dword(&f, cursor + 4) as usize;
            cursor += 8;
            for i in 0..n {
                assert_eq!(pcm[start + i], f[cursor + i], "PCM byte {}", start + i)
            }
            cursor += n;
        }
        assert_eq!(
            &Sha256::digest(&pcm)[..],
            &f[8..40],
            "complete original fixed-cadence PCM digest"
        );
        assert!(m.ended());
        assert_eq!(m.position(), 0);
        assert_eq!(m.render(64), vec![0; 128]);
    }
    #[test]
    #[ignore = "requires locally supplied PANIC_DATA; run with --ignored"]
    fn render_is_independent_of_host_callback_sizes() {
        let a = player().render(8192);
        let mut m = player();
        let mut b = Vec::new();
        for n in [1, 17, 511, 256, 4096, 3311] {
            b.extend(m.render(n));
        }
        assert_eq!(a, b);
    }
    #[test]
    #[ignore = "requires locally supplied PANIC_DATA and PANIC_FIXTURES; run with --ignored"]
    fn intro_cue_pcm_matches_original_dispatcher_at_controlled_arrivals() {
        let fixture = crate::test_support::fixture("tests/fixtures/music-cues.bin");
        let frames = dword(&fixture, 0) as usize;
        let mut m = player();
        let mut pcm = Vec::new();
        for frame in 0..frames {
            for cue in [58, 71, 80, 91, 103, 113, 140, 152, 165, 180] {
                if frame == 231 + 2 * cue {
                    m.intro_cue(cue)
                }
            }
            let n = (frame + 1) * 16000 / 70 - frame * 16000 / 70;
            pcm.extend(
                m.render(n)
                    .into_iter()
                    .map(|v| ((v as i32 / 256) + 128) as u8),
            );
        }
        assert_eq!(pcm.len(), fixture.len() - 4);
        for (i, (&actual, &expected)) in pcm.iter().zip(&fixture[4..]).enumerate() {
            assert_eq!(actual, expected, "cue PCM byte {i}")
        }
    }
    #[test]
    #[ignore = "requires locally supplied PANIC_DATA; run with --ignored"]
    fn all_intro_cues_have_original_routing_and_volumes() {
        let mut m = player();
        for (counter, frame) in [58, 71, 80, 91, 103, 113, 140, 152, 165, 180]
            .iter()
            .enumerate()
        {
            m.intro_cue(*frame);
            let first = 6 + (counter & 1);
            for (ch, v) in [
                (first, counter * 60 / 9),
                (first + 8, (9 - counter) * 60 / 9),
            ] {
                assert_eq!(&m.channels[ch].r[0x32..0x37], &[86, 47, v as u8, 0, 0]);
                assert_eq!(m.channels[ch].w(0x12), 604);
            }
        }
    }
}
