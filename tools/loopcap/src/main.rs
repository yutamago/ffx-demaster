//! loopcap <pid>|all <out.wav> <seconds>: captures the audio of one process tree (`all`: every
//! process but this one) through WASAPI process loopback (Windows 10 2004+) as 48 kHz stereo
//! f32, with the QPC time of every packet in
//! <out.wav>.ts ("packet <first frame> <frames> <qpc 100ns> <flags>") and clock pairs
//! ("clock <qpc 100ns> <filetime 100ns>") at the start and the end, so the samples can be put on
//! GetSystemTimePreciseAsFileTime's time (FFX.exe's Phyre_PTimerWin32_GetTime).
#[cfg(windows)]
fn main() -> windows::core::Result<()> {
    use std::io::Write;
    use std::sync::{Arc, Condvar, Mutex};
    use windows::Win32::Media::Audio::*;
    use windows::Win32::System::Com::StructuredStorage::*;
    use windows::Win32::System::Com::*;
    use windows::Win32::System::Performance::*;
    use windows::Win32::System::SystemInformation::GetSystemTimePreciseAsFileTime;
    use windows::Win32::System::Threading::*;
    use windows::Win32::System::Variant::VT_BLOB;
    use windows::core::*;

    let args: Vec<String> = std::env::args().collect();
    let all = args[1] == "all";
    let pid: u32 = if all { std::process::id() } else { args[1].parse().expect("pid") };
    let out = &args[2];
    let secs: f64 = args[3].parse().expect("seconds");

    #[implement(IActivateAudioInterfaceCompletionHandler)]
    struct Done(Arc<(Mutex<bool>, Condvar)>);
    impl IActivateAudioInterfaceCompletionHandler_Impl for Done_Impl {
        fn ActivateCompleted(&self, _op: Ref<IActivateAudioInterfaceAsyncOperation>) -> Result<()> {
            let (m, c) = &*self.0;
            *m.lock().unwrap() = true;
            c.notify_all();
            Ok(())
        }
    }

    let clock = || -> (u64, u64) {
        unsafe {
            let mut q = 0i64;
            let mut f = 0i64;
            QueryPerformanceCounter(&mut q).ok();
            QueryPerformanceFrequency(&mut f).ok();
            let ft = GetSystemTimePreciseAsFileTime();
            let q100 = (q as i128 * 10_000_000 / f as i128) as u64;
            (q100, ((ft.dwHighDateTime as u64) << 32) | ft.dwLowDateTime as u64)
        }
    };

    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
        let params = AUDIOCLIENT_ACTIVATION_PARAMS {
            ActivationType: AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK,
            Anonymous: AUDIOCLIENT_ACTIVATION_PARAMS_0 {
                ProcessLoopbackParams: AUDIOCLIENT_PROCESS_LOOPBACK_PARAMS {
                    TargetProcessId: pid,
                    ProcessLoopbackMode: if all { PROCESS_LOOPBACK_MODE_EXCLUDE_TARGET_PROCESS_TREE } else { PROCESS_LOOPBACK_MODE_INCLUDE_TARGET_PROCESS_TREE },
                },
            },
        };
        let mut pv = PROPVARIANT::default();
        {
            let inner = &mut *pv.Anonymous.Anonymous;
            inner.vt = VT_BLOB;
            inner.Anonymous.blob.cbSize = std::mem::size_of::<AUDIOCLIENT_ACTIVATION_PARAMS>() as u32;
            inner.Anonymous.blob.pBlobData = &params as *const _ as *mut u8;
        }
        let sync = Arc::new((Mutex::new(false), Condvar::new()));
        let handler: IActivateAudioInterfaceCompletionHandler = Done(sync.clone()).into();
        let op = ActivateAudioInterfaceAsync(
            VIRTUAL_AUDIO_DEVICE_PROCESS_LOOPBACK,
            &IAudioClient::IID,
            Some(&pv),
            &handler,
        )?;
        {
            let (m, c) = &*sync;
            let mut g = m.lock().unwrap();
            while !*g {
                g = c.wait(g).unwrap();
            }
        }
        let mut hr = HRESULT(0);
        let mut unk: Option<IUnknown> = None;
        op.GetActivateResult(&mut hr, &mut unk)?;
        hr.ok()?;
        let client: IAudioClient = unk.unwrap().cast()?;
        let fmt = WAVEFORMATEX {
            wFormatTag: 3, // WAVE_FORMAT_IEEE_FLOAT
            nChannels: 2,
            nSamplesPerSec: 48000,
            nAvgBytesPerSec: 48000 * 8,
            nBlockAlign: 8,
            wBitsPerSample: 32,
            cbSize: 0,
        };
        client.Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            AUDCLNT_STREAMFLAGS_LOOPBACK | AUDCLNT_STREAMFLAGS_EVENTCALLBACK | AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM,
            200_000,
            0,
            &fmt,
            None,
        )?;
        let ev = CreateEventW(None, false, false, None)?;
        client.SetEventHandle(ev)?;
        let cap: IAudioCaptureClient = client.GetService()?;
        let mut wav = std::io::BufWriter::new(std::fs::File::create(out).unwrap());
        let mut ts = std::io::BufWriter::new(std::fs::File::create(format!("{out}.ts")).unwrap());
        wav.write_all(&[0u8; 44]).unwrap();
        let (q, f) = clock();
        writeln!(ts, "clock {q} {f}").unwrap();
        client.Start()?;
        let t0 = std::time::Instant::now();
        let mut total: u64 = 0;
        while t0.elapsed().as_secs_f64() < secs {
            WaitForSingleObject(ev, 100);
            loop {
                let n = cap.GetNextPacketSize()?;
                if n == 0 {
                    break;
                }
                let mut data = std::ptr::null_mut();
                let mut frames = 0u32;
                let mut flags = 0u32;
                let mut dev = 0u64;
                let mut qpc = 0u64;
                cap.GetBuffer(&mut data, &mut frames, &mut flags, Some(&mut dev), Some(&mut qpc))?;
                let bytes = frames as usize * 8;
                if flags & (AUDCLNT_BUFFERFLAGS_SILENT.0 as u32) != 0 || data.is_null() {
                    wav.write_all(&vec![0u8; bytes]).unwrap();
                } else {
                    wav.write_all(std::slice::from_raw_parts(data, bytes)).unwrap();
                }
                writeln!(ts, "packet {total} {frames} {qpc} {flags} {dev}").unwrap();
                total += frames as u64;
                cap.ReleaseBuffer(frames)?;
            }
        }
        client.Stop()?;
        let (q, f) = clock();
        writeln!(ts, "clock {q} {f}").unwrap();
        // WAV header
        let data_len = (total * 8) as u32;
        let mut h = Vec::new();
        h.extend_from_slice(b"RIFF");
        h.extend_from_slice(&(36 + data_len).to_le_bytes());
        h.extend_from_slice(b"WAVEfmt ");
        h.extend_from_slice(&16u32.to_le_bytes());
        h.extend_from_slice(&3u16.to_le_bytes());
        h.extend_from_slice(&2u16.to_le_bytes());
        h.extend_from_slice(&48000u32.to_le_bytes());
        h.extend_from_slice(&(48000u32 * 8).to_le_bytes());
        h.extend_from_slice(&8u16.to_le_bytes());
        h.extend_from_slice(&32u16.to_le_bytes());
        h.extend_from_slice(b"data");
        h.extend_from_slice(&data_len.to_le_bytes());
        use std::io::Seek;
        let mut file = wav.into_inner().unwrap();
        file.seek(std::io::SeekFrom::Start(0)).unwrap();
        file.write_all(&h).unwrap();
        println!("captured {total} frames ({:.2} s)", total as f64 / 48000.0);
    }
    Ok(())
}

#[cfg(not(windows))]
fn main() {
    eprintln!("loopcap is a Windows program");
}
