using System.Reflection;
using System.Runtime.InteropServices;

namespace PolicyEngine;

// The C ABI exported by the policy_ffi Rust cdylib.
//
//   void* policy_engine_load(const uint8_t* wasm, uintptr_t len);
//   int32 policy_on_request(void* handle, const FfiRequest* req, FfiDecision* out);
//   void  policy_decision_free(FfiDecision* out);
//   void  policy_engine_free(void* handle);
//   const char* policy_last_error(void);
//
// Request strings are null-terminated UTF-8. policy_on_request returns 0 on
// success, nonzero on error. The decision's message is owned by the callee
// and freed with policy_decision_free.
internal static partial class NativeMethods
{
    private const string Lib = "policy_ffi";

    [StructLayout(LayoutKind.Sequential)]
    internal struct FfiHeader
    {
        public IntPtr Name;
        public IntPtr Value;
    }

    [StructLayout(LayoutKind.Sequential)]
    internal struct FfiRequest
    {
        public IntPtr Method;
        public IntPtr Path;
        public IntPtr Headers;
        public nuint HeadersLen;
    }

    [StructLayout(LayoutKind.Sequential)]
    internal struct FfiDecision
    {
        public int Kind; // 0 = allow, 1 = reject
        public ushort Status;
        public IntPtr Message;
    }

    static NativeMethods()
    {
        NativeLibrary.SetDllImportResolver(typeof(NativeMethods).Assembly, Resolve);
    }

    // Resolve the cdylib from POLICY_FFI_LIB if set, else fall back to the
    // default OS search path.
    private static IntPtr Resolve(string name, Assembly assembly, DllImportSearchPath? path)
    {
        if (name != Lib)
            return IntPtr.Zero;

        var explicitPath = Environment.GetEnvironmentVariable("POLICY_FFI_LIB");
        if (!string.IsNullOrEmpty(explicitPath) && NativeLibrary.TryLoad(explicitPath, out var handle))
            return handle;

        return NativeLibrary.TryLoad(name, assembly, path, out handle) ? handle : IntPtr.Zero;
    }

    [LibraryImport(Lib)]
    internal static partial IntPtr policy_engine_load(byte[] wasm, nuint len);

    [LibraryImport(Lib)]
    internal static partial int policy_on_request(IntPtr handle, in FfiRequest req, out FfiDecision decision);

    [LibraryImport(Lib)]
    internal static partial void policy_decision_free(ref FfiDecision decision);

    [LibraryImport(Lib)]
    internal static partial void policy_engine_free(IntPtr handle);

    [LibraryImport(Lib)]
    internal static partial IntPtr policy_last_error();
}
