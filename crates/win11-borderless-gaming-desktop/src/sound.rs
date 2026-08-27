use std::{mem, ptr};

use windows::{
    Win32::Media::{
        Audio::{
            CALLBACK_NULL, HWAVEOUT, PlaySoundA, SND_ASYNC, SND_MEMORY, SND_NODEFAULT, SND_NOSTOP,
            WAVE_FORMAT_PCM, WAVE_MAPPER, WAVEFORMATEX, WAVEHDR, WHDR_DONE, waveOutClose,
            waveOutOpen, waveOutPrepareHeader, waveOutReset, waveOutUnprepareHeader, waveOutWrite,
        },
        MMSYSERR_NOERROR,
    },
    core::{PCSTR, PSTR},
};

const WAV_HEADER_BYTES: usize = 44;
pub(crate) const WARM_UP_MILLISECONDS: usize = 160;
const WARM_UP_PCM_BYTES: usize = 48_000 * 2 * WARM_UP_MILLISECONDS / 1_000;
static WARM_UP_PCM: [u8; WARM_UP_PCM_BYTES] = [0; WARM_UP_PCM_BYTES];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SoundCue {
    Countdown,
    GamingEnter,
    GamingLeave,
}

impl SoundCue {
    fn wav(self) -> &'static [u8] {
        match self {
            Self::Countdown => include_bytes!("../assets/runtime/countdown.wav"),
            Self::GamingEnter => include_bytes!("../assets/runtime/gaming-enter.wav"),
            Self::GamingLeave => include_bytes!("../assets/runtime/gaming-leave.wav"),
        }
    }

    fn pcm(self) -> &'static [u8] {
        &self.wav()[WAV_HEADER_BYTES..]
    }
}

struct PendingBuffer {
    header: Box<WAVEHDR>,
    audible: bool,
}

struct WaveOutPlayer {
    handle: HWAVEOUT,
    pending: Vec<PendingBuffer>,
}

impl WaveOutPlayer {
    fn open() -> Option<Self> {
        let format = WAVEFORMATEX {
            wFormatTag: WAVE_FORMAT_PCM as u16,
            nChannels: 1,
            nSamplesPerSec: 48_000,
            nAvgBytesPerSec: 48_000 * 2,
            nBlockAlign: 2,
            wBitsPerSample: 16,
            cbSize: 0,
        };
        let mut handle = HWAVEOUT::default();

        // SAFETY: `format` and `handle` remain valid for this synchronous call.
        // CALLBACK_NULL reports asynchronous completion through WAVEHDR flags.
        let result = unsafe {
            waveOutOpen(
                Some(&mut handle),
                WAVE_MAPPER,
                ptr::from_ref(&format),
                None,
                None,
                CALLBACK_NULL,
            )
        };
        (result == MMSYSERR_NOERROR).then_some(Self {
            handle,
            pending: Vec::with_capacity(2),
        })
    }

    fn queue(&mut self, pcm: &'static [u8], audible: bool) -> bool {
        let Ok(buffer_length) = u32::try_from(pcm.len()) else {
            return false;
        };
        let mut header = Box::new(WAVEHDR {
            lpData: PSTR(pcm.as_ptr() as *mut u8),
            dwBufferLength: buffer_length,
            ..Default::default()
        });
        let header_size = mem::size_of::<WAVEHDR>() as u32;

        // SAFETY: the boxed header has a stable address and `pcm` is embedded
        // static data. Both remain alive in `pending` until WinMM is done.
        if unsafe { waveOutPrepareHeader(self.handle, header.as_mut(), header_size) }
            != MMSYSERR_NOERROR
        {
            return false;
        }
        if unsafe { waveOutWrite(self.handle, header.as_mut(), header_size) } != MMSYSERR_NOERROR {
            // SAFETY: this header was prepared but never accepted for playback.
            if unsafe { waveOutUnprepareHeader(self.handle, header.as_mut(), header_size) }
                != MMSYSERR_NOERROR
            {
                // Retaining memory is safer than freeing a header unexpectedly
                // still owned by a driver. Process shutdown will reclaim it.
                mem::forget(header);
            }
            return false;
        }

        self.pending.push(PendingBuffer { header, audible });
        true
    }

    fn reap_finished(&mut self) {
        let mut index = 0;
        while index < self.pending.len() {
            // SAFETY: volatile access observes the driver's asynchronous flag
            // write without borrowing the packed field at an unaligned address.
            let flags =
                unsafe { ptr::addr_of!(self.pending[index].header.dwFlags).read_volatile() };
            if flags & WHDR_DONE == 0 {
                index += 1;
                continue;
            }

            // SAFETY: WHDR_DONE means the driver has returned the stable header.
            let unprepared = unsafe {
                waveOutUnprepareHeader(
                    self.handle,
                    self.pending[index].header.as_mut(),
                    mem::size_of::<WAVEHDR>() as u32,
                )
            } == MMSYSERR_NOERROR;
            if unprepared {
                self.pending.remove(index);
            } else {
                index += 1;
            }
        }
    }

    fn has_audible_buffer(&self) -> bool {
        self.pending.iter().any(|buffer| buffer.audible)
    }
}

impl Drop for WaveOutPlayer {
    fn drop(&mut self) {
        // SAFETY: reset synchronously returns all queued buffers before cleanup.
        let _ = unsafe { waveOutReset(self.handle) };
        for mut pending in self.pending.drain(..) {
            // SAFETY: reset above returned the buffer to this sole owner.
            if unsafe {
                waveOutUnprepareHeader(
                    self.handle,
                    pending.header.as_mut(),
                    mem::size_of::<WAVEHDR>() as u32,
                )
            } != MMSYSERR_NOERROR
            {
                mem::forget(pending);
            }
        }
        // SAFETY: all successfully returned headers were unprepared; this is
        // the sole owner of the wave-output handle.
        let _ = unsafe { waveOutClose(self.handle) };
    }
}

#[derive(Default)]
pub(crate) struct SoundPlayer {
    player: Option<WaveOutPlayer>,
}

impl SoundPlayer {
    fn player(&mut self) -> Option<&mut WaveOutPlayer> {
        if self.player.is_none() {
            self.player = WaveOutPlayer::open();
        }
        self.player.as_mut()
    }

    /// Opens and exercises the output during the button animation, then queues
    /// digit 3 on the same handle with no cold-start gap.
    pub(crate) fn prepare_countdown(&mut self) -> bool {
        let Some(player) = self.player() else {
            return false;
        };
        player.reap_finished();
        if !player.pending.is_empty() {
            return false;
        }
        if player.queue(&WARM_UP_PCM, false) && player.queue(SoundCue::Countdown.pcm(), true) {
            return true;
        }

        self.player = None;
        false
    }

    /// Primes restoration audio without keeping a looping silent stream alive.
    pub(crate) fn warm_up(&mut self) {
        let Some(player) = self.player() else {
            return;
        };
        player.reap_finished();
        if player.pending.is_empty() && !player.queue(&WARM_UP_PCM, false) {
            self.player = None;
        }
    }

    pub(crate) fn play(&mut self, cue: SoundCue) {
        let queued = if let Some(player) = self.player() {
            player.reap_finished();
            if player.has_audible_buffer() {
                // Preserve the old SND_NOSTOP behavior for audible cues.
                true
            } else {
                player.queue(cue.pcm(), true)
            }
        } else {
            false
        };
        if queued {
            return;
        }

        self.player = None;
        let wav = cue.wav();
        // SAFETY: fallback SND_MEMORY reads the embedded WAVE image, whose
        // static buffer remains valid throughout asynchronous playback.
        let _ = unsafe {
            PlaySoundA(
                PCSTR::from_raw(wav.as_ptr()),
                None,
                SND_MEMORY | SND_ASYNC | SND_NODEFAULT | SND_NOSTOP,
            )
        };
    }
}

pub(crate) const fn transition_cue(was_gaming: bool, is_gaming: bool) -> Option<SoundCue> {
    match (was_gaming, is_gaming) {
        (false, true) => Some(SoundCue::GamingEnter),
        (true, false) => Some(SoundCue::GamingLeave),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_transition_cues_only_follow_real_state_changes() {
        assert_eq!(transition_cue(false, false), None);
        assert_eq!(transition_cue(false, true), Some(SoundCue::GamingEnter));
        assert_eq!(transition_cue(true, false), Some(SoundCue::GamingLeave));
        assert_eq!(transition_cue(true, true), None);
    }

    #[test]
    fn warm_up_matches_the_click_animation_and_cue_pcm_format() {
        assert_eq!(WARM_UP_PCM.len(), 48_000 * 2 * 160 / 1_000);
        assert!(WARM_UP_PCM.iter().all(|sample| *sample == 0));
        for cue in [
            SoundCue::Countdown,
            SoundCue::GamingEnter,
            SoundCue::GamingLeave,
        ] {
            assert_eq!(&cue.wav()[0..4], b"RIFF");
            assert_eq!(cue.pcm().len(), cue.wav().len() - WAV_HEADER_BYTES);
        }
    }
}
