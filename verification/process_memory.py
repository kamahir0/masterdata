"""OS-reported peak memory, with an explicit process boundary for evidence."""
import ctypes
import platform


def windows_peak(process):
    class Counters(ctypes.Structure):
        _fields_ = [('cb', ctypes.c_ulong), ('PageFaultCount', ctypes.c_ulong)] + [
            (name, ctypes.c_size_t) for name in ['PeakWorkingSetSize', 'WorkingSetSize',
            'QuotaPeakPagedPoolUsage', 'QuotaPagedPoolUsage', 'QuotaPeakNonPagedPoolUsage',
            'QuotaNonPagedPoolUsage', 'PagefileUsage', 'PeakPagefileUsage']]
    counters = Counters()
    counters.cb = ctypes.sizeof(counters)
    call = ctypes.windll.psapi.GetProcessMemoryInfo
    call.argtypes = [ctypes.c_void_p, ctypes.POINTER(Counters), ctypes.c_ulong]
    call.restype = ctypes.c_int
    if call(ctypes.c_void_p(int(process._handle)), ctypes.byref(counters), counters.cb):
        return counters.PeakWorkingSetSize
    return 0


def child_peak(observed):
    if platform.system() == 'Windows':
        return observed, 'maximum OS PeakWorkingSetSize reported while native process alive; excludes WebView subprocesses'
    import resource
    peak = resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss
    if platform.system() != 'Darwin':
        peak *= 1024
    return peak, 'maximum OS child lifetime peak in isolated runner; not an aggregate of native and WebView subprocesses'
