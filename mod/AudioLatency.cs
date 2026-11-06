// SPDX-License-Identifier: MIT

using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

namespace Yuta.Demaster;

/// <summary>
/// audio.latency = "ps2": FMOD's mixer runs with 256-sample DSP buffers (x 4, 21 ms at 48 kHz)
/// instead of FMOD Ex's default 1024 x 4 (85 ms), so every sound, voices included, reaches the
/// speakers about 60 ms sooner (docs/timing.md, "Voice start").
///
/// FfxFmod_initEventSystem (0x00706820, on the FMOD thread, FMOD command 0x2a) creates the
/// EventSystem and calls EventSystem::init(128, 0, NULL, 0) through FFX.exe's import slot without
/// ever setting the buffer size. This puts a wrapper into that slot: it asks the EventSystem for
/// its low-level System and calls System::setDSPBufferSize before the original init (FMOD takes
/// the buffer size only before System::init).
/// DEMASTER_DSP_BUFFER=&lt;length&gt;x&lt;count&gt; (environment) picks another size, for measurements.
/// </summary>
internal static unsafe class AudioLatency {
    private const string FMOD_EVENT = "fmod_event.dll";
    private const string FMODEX     = "fmodex.dll";
    private const string INIT       = "?init@EventSystem@FMOD@@QAG?AW4FMOD_RESULT@@HIPAXI@Z";
    private const string GET_SYSTEM = "?getSystemObject@EventSystem@FMOD@@QAG?AW4FMOD_RESULT@@PAPAVSystem@2@@Z";
    private const string SET_DSP    = "?setDSPBufferSize@System@FMOD@@QAG?AW4FMOD_RESULT@@IH@Z";
    private const string GET_DSP    = "?getDSPBufferSize@System@FMOD@@QAG?AW4FMOD_RESULT@@PAIPAH@Z";
    private const string GET_OUTPUT = "?getOutput@System@FMOD@@QAG?AW4FMOD_RESULT@@PAW4FMOD_OUTPUTTYPE@@@Z";

    private const uint PAGE_READWRITE = 0x04;

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode)]
    private static extern nint GetModuleHandleW(string? name);

    [DllImport("kernel32.dll", CharSet = CharSet.Ansi)]
    private static extern nint GetProcAddress(nint module, string name);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool VirtualProtect(nint address, nuint size, uint protect, out uint old);

    private static FhLogger? _log;
    private static nint _original;
    private static uint _length;
    private static int  _count;

    public static bool apply(DemasterModule m, string value) {
        _log = m.log;
        (_length, _count) = (256, 4);
        string? env = Environment.GetEnvironmentVariable("DEMASTER_DSP_BUFFER");
        if (!string.IsNullOrEmpty(env)) {
            string[] p = env.Split('x');
            if (p.Length == 2 && uint.TryParse(p[0], out uint l) && int.TryParse(p[1], out int c) && l >= 64 && c >= 2) (_length, _count) = (l, c);
            else m.log.Warning($"audio.latency: DEMASTER_DSP_BUFFER={env} is not <length>x<count>; using 256x4");
        }
        nint slot = import_slot(GetModuleHandleW(null), FMOD_EVENT, INIT);
        if (slot == 0) {
            m.log.Warning($"audio.latency: FFX.exe imports no {INIT} from {FMOD_EVENT}; not patched");
            return false;
        }
        nint now = *(nint*)slot;
        delegate* unmanaged[Stdcall]<nint, int, uint, nint, uint, int> wrapper = &init;
        if (now == (nint)wrapper) return true;
        if (!VirtualProtect(slot, (nuint)sizeof(nint), PAGE_READWRITE, out uint old)) {
            m.log.Warning($"audio.latency: VirtualProtect failed ({Marshal.GetLastWin32Error()})");
            return false;
        }
        _original = now;
        *(nint*)slot = (nint)wrapper;
        VirtualProtect(slot, (nuint)sizeof(nint), old, out _);
        m.log.Info($"audio.latency: EventSystem::init's import slot 0x{slot:x8} wrapped; DSP buffer {_length} x {_count} at FMOD's start");
        return true;
    }

    /// <summary>EventSystem::init(this, maxchannels, flags, extradriverdata, eventflags), stdcall.</summary>
    [UnmanagedCallersOnly(CallConvs = [typeof(CallConvStdcall)])]
    private static int init(nint es, int channels, uint flags, nint extra, uint event_flags) {
        nint system = 0;
        int set = -1;
        try {
            var get = (delegate* unmanaged[Stdcall]<nint, nint*, int>)fmod(FMOD_EVENT, GET_SYSTEM);
            var dsp = (delegate* unmanaged[Stdcall]<nint, uint, int, int>)fmod(FMODEX, SET_DSP);
            if (get != null && dsp != null && get(es, &system) == 0 && system != 0) set = dsp(system, _length, _count);
        }
        catch (Exception e) { _log?.Error($"audio.latency: {e}"); }
        int r = ((delegate* unmanaged[Stdcall]<nint, int, uint, nint, uint, int>)_original)(es, channels, flags, extra, event_flags);
        try {
            uint length = 0; int count = 0, output = -1;
            var gdsp = (delegate* unmanaged[Stdcall]<nint, uint*, int*, int>)fmod(FMODEX, GET_DSP);
            var gout = (delegate* unmanaged[Stdcall]<nint, int*, int>)fmod(FMODEX, GET_OUTPUT);
            if (system != 0 && gdsp != null) gdsp(system, &length, &count);
            if (system != 0 && gout != null) gout(system, &output);
            _log?.Info($"audio.latency: setDSPBufferSize({_length}, {_count}) = {set}; EventSystem::init = {r}; FMOD runs {length} x {count} samples, output type {output}");
        }
        catch (Exception e) { _log?.Error($"audio.latency: {e}"); }
        return r;
    }

    private static nint fmod(string dll, string name) {
        nint h = GetModuleHandleW(dll);
        return h == 0 ? 0 : GetProcAddress(h, name);
    }

    /// <summary>The import address table entry of <paramref name="dll"/>!<paramref name="name"/> in the module at <paramref name="b"/>.</summary>
    private static nint import_slot(nint b, string dll, string name) {
        byte* image = (byte*)b;
        byte* nt = image + *(int*)(image + 0x3c);
        // PE32: the optional header at +0x18, its data directory 1 (imports) at +0x60 + 8
        uint dir = *(uint*)(nt + 0x18 + 0x60 + 8);
        if (dir == 0) return 0;
        for (uint* d = (uint*)(image + dir); d[3] != 0; d += 5) {
            string lib = Marshal.PtrToStringAnsi((nint)(image + d[3])) ?? "";
            if (!lib.Equals(dll, StringComparison.OrdinalIgnoreCase)) continue;
            uint* names = (uint*)(image + (d[0] != 0 ? d[0] : d[4]));
            for (int i = 0; names[i] != 0; i++) {
                if ((names[i] & 0x8000_0000) != 0) continue;
                string n = Marshal.PtrToStringAnsi((nint)(image + names[i] + 2)) ?? "";
                if (n == name) return (nint)(image + d[4] + 4 * i);
            }
        }
        return 0;
    }
}
