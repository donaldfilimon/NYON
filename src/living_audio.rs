//! Original procedural supporting audio. Wall time and levels are presentation
//! only; hardware failure never affects an authority or its load path.
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, time::Duration};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AudioChannel {
    Master,
    Ambience,
    Music,
    Effects,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingAudioSettings {
    pub muted: bool,
    pub master: u8,
    pub ambience: u8,
    pub music: u8,
    pub effects: u8,
}
impl Default for LivingAudioSettings {
    fn default() -> Self {
        Self {
            muted: false,
            master: 50,
            ambience: 20,
            music: 15,
            effects: 50,
        }
    }
}
impl LivingAudioSettings {
    pub fn validate(self) -> bool {
        [self.master, self.ambience, self.music, self.effects]
            .into_iter()
            .all(|value| value <= 100)
    }
    pub fn cycle(&mut self, channel: AudioChannel) {
        let level = match channel {
            AudioChannel::Master => &mut self.master,
            AudioChannel::Ambience => &mut self.ambience,
            AudioChannel::Music => &mut self.music,
            AudioChannel::Effects => &mut self.effects,
        };
        *level = match *level {
            0 => 25,
            1..=25 => 50,
            26..=50 => 75,
            51..=75 => 100,
            _ => 0,
        };
    }
    fn gain(self, channel: AudioChannel) -> f32 {
        if self.muted {
            return 0.0;
        }
        let level = match channel {
            AudioChannel::Master => 100,
            AudioChannel::Ambience => self.ambience,
            AudioChannel::Music => self.music,
            AudioChannel::Effects => self.effects,
        };
        f32::from(self.master) * f32::from(level) / 10_000.0
    }
}
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum AudioCue {
    Selection,
    Construction,
    Hazard,
    Conflict,
}
impl AudioCue {
    fn frequency(self) -> f32 {
        match self {
            Self::Selection => 440.0,
            Self::Construction => 660.0,
            Self::Hazard => 220.0,
            Self::Conflict => 165.0,
        }
    }
}

/// One category may start once within a rolling wall-second; all categories together
/// are capped at four. A scene change resets presentation, never authority.
#[derive(Default)]
pub struct AudioLimiter {
    starts: VecDeque<(Duration, AudioCue)>,
}
impl AudioLimiter {
    pub fn admit(&mut self, elapsed: Duration, cue: AudioCue) -> bool {
        while self
            .starts
            .front()
            .is_some_and(|(started, _)| elapsed.saturating_sub(*started) >= Duration::from_secs(1))
        {
            self.starts.pop_front();
        }
        if self.starts.len() >= 4 || self.starts.iter().any(|(_, prior)| *prior == cue) {
            return false;
        }
        self.starts.push_back((elapsed, cue));
        true
    }
}

#[derive(Default)]
pub struct LivingAudio {
    pub settings: LivingAudioSettings,
    limiter: AudioLimiter,
    output: Output,
    active: bool,
    gesture: bool,
    #[cfg(not(target_arch = "wasm32"))]
    settings_path: Option<std::path::PathBuf>,
}
impl LivingAudio {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_native(&mut self, path: std::path::PathBuf) {
        use std::io::Read;
        if let Ok(file) = std::fs::File::open(&path) {
            let mut bytes = Vec::new();
            if file.take(1025).read_to_end(&mut bytes).is_ok()
                && bytes.len() <= 1024
                && let Ok(settings) = serde_json::from_slice::<LivingAudioSettings>(&bytes)
                && settings.validate()
            {
                self.settings = settings;
            }
        }
        self.settings_path = Some(path);
    }
    #[cfg(target_arch = "wasm32")]
    pub fn load_browser(&mut self) {
        if let Some(text) = web_sys::window()
            .and_then(|window| window.local_storage().ok().flatten())
            .and_then(|store| store.get_item("nyon.living.audio.v1").ok().flatten())
            && text.len() <= 1024
            && let Ok(settings) = serde_json::from_str::<LivingAudioSettings>(&text)
            && settings.validate()
        {
            self.settings = settings;
        }
    }
    pub fn persist(&self) -> Result<(), String> {
        let bytes = serde_json::to_vec(&self.settings)
            .map_err(|_| "Audio settings could not be encoded")?;
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::io::Write;
            let path = self
                .settings_path
                .as_ref()
                .ok_or("Audio settings storage unavailable")?;
            let parent = path.parent().ok_or("Audio settings storage unavailable")?;
            std::fs::create_dir_all(parent).map_err(|_| "Audio settings storage unavailable")?;
            let mut file = tempfile::NamedTempFile::new_in(parent)
                .map_err(|_| "Audio settings write failed")?;
            file.write_all(&bytes)
                .and_then(|()| file.as_file().sync_all())
                .map_err(|_| "Audio settings write failed")?;
            file.persist(path)
                .map_err(|_| "Audio settings placement failed")?;
            #[cfg(unix)]
            std::fs::File::open(parent)
                .and_then(|dir| dir.sync_all())
                .map_err(|_| "Audio settings directory sync failed")?;
            Ok(())
        }
        #[cfg(target_arch = "wasm32")]
        {
            let store = web_sys::window()
                .and_then(|window| window.local_storage().ok().flatten())
                .ok_or("Audio settings storage unavailable")?;
            store
                .set_item(
                    "nyon.living.audio.v1",
                    std::str::from_utf8(&bytes).map_err(|_| "Audio settings encoding failed")?,
                )
                .map_err(|_| "Audio settings write failed")
                .map_err(str::to_owned)
        }
    }
    pub fn user_gesture(&mut self) {
        self.gesture = true;
        self.output.ensure();
    }
    pub fn scene(&mut self, active: bool) {
        if self.active != active {
            self.output.stop();
            self.active = active;
            self.limiter = AudioLimiter::default();
        }
        if active && self.gesture {
            self.output.layers(self.settings);
        }
    }
    pub fn refresh(&mut self) {
        self.output.stop();
        if self.active && self.gesture {
            self.output.layers(self.settings);
        }
    }
    pub fn cue(&mut self, elapsed: Duration, cue: AudioCue) {
        if self.active && self.gesture && !self.settings.muted && self.limiter.admit(elapsed, cue) {
            self.output
                .cue(cue, self.settings.gain(AudioChannel::Effects));
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Default)]
struct Output {
    device: Option<rodio::MixerDeviceSink>,
    attempted: bool,
    layers: Vec<rodio::Player>,
    effects: Vec<rodio::Player>,
}
#[cfg(not(target_arch = "wasm32"))]
impl Output {
    fn ensure(&mut self) {
        if !self.attempted {
            self.attempted = true;
            self.device = rodio::DeviceSinkBuilder::open_default_sink().ok();
            if let Some(device) = &mut self.device {
                device.log_on_drop(false);
            }
        }
    }
    fn stop(&mut self) {
        self.layers.clear();
        self.effects.clear();
    }
    fn layers(&mut self, settings: LivingAudioSettings) {
        use rodio::Source;
        if !self.layers.is_empty() {
            return;
        }
        let Some(device) = &self.device else {
            return;
        };
        for (frequency, channel) in [(82.4, AudioChannel::Ambience), (123.5, AudioChannel::Music)] {
            let player = rodio::Player::connect_new(device.mixer());
            player.set_volume(settings.gain(channel) * 0.018);
            player.append(rodio::source::SineWave::new(frequency).fade_in(Duration::from_secs(2)));
            self.layers.push(player);
        }
    }
    fn cue(&mut self, cue: AudioCue, gain: f32) {
        use rodio::Source;
        self.effects.retain(|player| !player.empty());
        let Some(device) = &self.device else {
            return;
        };
        let player = rodio::Player::connect_new(device.mixer());
        player.set_volume(gain * 0.06);
        player.append(
            rodio::source::SineWave::new(cue.frequency())
                .take_duration(Duration::from_millis(160))
                .fade_in(Duration::from_millis(15))
                .fade_out(Duration::from_millis(80)),
        );
        self.effects.push(player);
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Default)]
struct Output {
    context: Option<web_sys::AudioContext>,
    layers: Vec<BrowserVoice>,
    effects: Vec<BrowserVoice>,
}
#[cfg(target_arch = "wasm32")]
struct BrowserVoice {
    oscillator: web_sys::OscillatorNode,
    gain: web_sys::GainNode,
    end: Option<f64>,
}
#[cfg(target_arch = "wasm32")]
impl BrowserVoice {
    fn stop(self) {
        let _ = self.oscillator.stop();
        let _ = self.oscillator.disconnect();
        let _ = self.gain.disconnect();
    }
}
#[cfg(target_arch = "wasm32")]
impl Output {
    fn ensure(&mut self) {
        if self.context.is_none() {
            self.context = web_sys::AudioContext::new().ok();
        }
        if let Some(context) = &self.context {
            let _ = context.resume();
        }
    }
    fn stop(&mut self) {
        for voice in self.layers.drain(..).chain(self.effects.drain(..)) {
            voice.stop();
        }
    }
    fn tone(&self, frequency: f32, gain: f32, duration: Option<f64>) -> Option<BrowserVoice> {
        let context = self.context.as_ref()?;
        let oscillator = context.create_oscillator().ok()?;
        let volume = context.create_gain().ok()?;
        oscillator.frequency().set_value(frequency);
        volume.gain().set_value(gain);
        oscillator.connect_with_audio_node(&volume).ok()?;
        volume
            .connect_with_audio_node(&context.destination())
            .ok()?;
        oscillator.start().ok()?;
        if let Some(duration) = duration {
            let now = context.current_time();
            let _ = volume
                .gain()
                .linear_ramp_to_value_at_time(0.0, now + duration);
            let _ = oscillator.stop_with_when(now + duration);
        }
        Some(BrowserVoice {
            oscillator,
            gain: volume,
            end: duration.map(|duration| context.current_time() + duration),
        })
    }
    fn layers(&mut self, settings: LivingAudioSettings) {
        let now = self
            .context
            .as_ref()
            .map_or(0.0, web_sys::AudioContext::current_time);
        let mut index = 0;
        while index < self.effects.len() {
            if self.effects[index].end.is_some_and(|end| end <= now) {
                self.effects.remove(index).stop();
            } else {
                index += 1;
            }
        }
        if self.layers.is_empty() {
            for (frequency, channel) in
                [(82.4, AudioChannel::Ambience), (123.5, AudioChannel::Music)]
            {
                if let Some(node) = self.tone(frequency, settings.gain(channel) * 0.018, None) {
                    self.layers.push(node);
                }
            }
        }
    }
    fn cue(&mut self, cue: AudioCue, gain: f32) {
        if self.effects.len() >= 4 {
            self.effects.remove(0).stop();
        }
        if let Some(voice) = self.tone(cue.frequency(), gain * 0.06, Some(0.16)) {
            self.effects.push(voice);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rate_limits_coalesce_and_mute_is_zero_at_every_level() {
        let mut limiter = AudioLimiter::default();
        assert!(limiter.admit(Duration::ZERO, AudioCue::Conflict));
        assert!(!limiter.admit(Duration::from_millis(999), AudioCue::Conflict));
        for cue in [
            AudioCue::Hazard,
            AudioCue::Selection,
            AudioCue::Construction,
        ] {
            assert!(limiter.admit(Duration::ZERO, cue));
        }
        assert!(limiter.admit(Duration::from_secs(1), AudioCue::Conflict));
        let settings = LivingAudioSettings {
            muted: true,
            ..LivingAudioSettings::default()
        };
        for channel in [
            AudioChannel::Master,
            AudioChannel::Ambience,
            AudioChannel::Music,
            AudioChannel::Effects,
        ] {
            assert_eq!(settings.gain(channel), 0.0);
        }
    }

    #[test]
    fn rate_limit_does_not_reset_at_integer_second_boundaries() {
        let mut limiter = AudioLimiter::default();
        for cue in [
            AudioCue::Conflict,
            AudioCue::Hazard,
            AudioCue::Selection,
            AudioCue::Construction,
        ] {
            assert!(limiter.admit(Duration::from_millis(999), cue));
            assert!(!limiter.admit(Duration::from_millis(1000), cue));
        }
        assert!(!limiter.admit(Duration::from_millis(1998), AudioCue::Conflict));
        assert!(limiter.admit(Duration::from_millis(1999), AudioCue::Conflict));
    }
}
