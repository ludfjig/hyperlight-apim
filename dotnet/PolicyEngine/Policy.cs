using System.Runtime.InteropServices;
using static PolicyEngine.NativeMethods;

namespace PolicyEngine;

/// A loaded customer policy backed by a Hyperlight sandbox. One instance
/// per customer. The native engine serializes calls internally, so the
/// sandbox call and its restore stay atomic.
public sealed class Policy : IDisposable
{
    private IntPtr _handle;

    private Policy(IntPtr handle) => _handle = handle;

    /// Load a policy component from raw wasm bytes.
    public static Policy Load(byte[] wasm)
    {
        var handle = policy_engine_load(wasm, (nuint)wasm.Length);
        if (handle == IntPtr.Zero)
            throw new PolicyException(LastError() ?? "policy_engine_load failed");
        return new Policy(handle);
    }

    /// Run the policy for one request. Throws PolicyException if the guest
    /// crashes or the engine errors.
    public Decision OnRequest(PolicyRequest request)
    {
        var strings = new List<IntPtr>();
        IntPtr Utf8(string s)
        {
            var p = Marshal.StringToCoTaskMemUTF8(s);
            strings.Add(p);
            return p;
        }

        var headerSize = Marshal.SizeOf<FfiHeader>();
        var headersPtr = IntPtr.Zero;
        try
        {
            var native = new FfiRequest
            {
                Method = Utf8(request.Method),
                Path = Utf8(request.Path),
                HeadersLen = (nuint)request.Headers.Count,
            };

            if (request.Headers.Count > 0)
            {
                headersPtr = Marshal.AllocCoTaskMem(headerSize * request.Headers.Count);
                for (var i = 0; i < request.Headers.Count; i++)
                {
                    var h = new FfiHeader
                    {
                        Name = Utf8(request.Headers[i].Name),
                        Value = Utf8(request.Headers[i].Value),
                    };
                    Marshal.StructureToPtr(h, headersPtr + i * headerSize, false);
                }
                native.Headers = headersPtr;
            }

            int rc;
            FfiDecision decision;
            if (_handle == IntPtr.Zero)
                throw new ObjectDisposedException(nameof(Policy));
            rc = policy_on_request(_handle, in native, out decision);

            if (rc != 0)
                throw new PolicyException(LastError() ?? "policy_on_request failed");

            try
            {
                if (decision.Kind == 0)
                    return new Decision { Allow = true };

                var message = decision.Message == IntPtr.Zero
                    ? null
                    : Marshal.PtrToStringUTF8(decision.Message);
                return new Decision { Allow = false, Status = decision.Status, Message = message };
            }
            finally
            {
                policy_decision_free(ref decision);
            }
        }
        finally
        {
            if (headersPtr != IntPtr.Zero)
                Marshal.FreeCoTaskMem(headersPtr);
            foreach (var p in strings)
                Marshal.FreeCoTaskMem(p);
        }
    }

    private static string? LastError()
    {
        var ptr = policy_last_error();
        return ptr == IntPtr.Zero ? null : Marshal.PtrToStringUTF8(ptr);
    }

    public void Dispose()
    {
        if (_handle != IntPtr.Zero)
        {
            policy_engine_free(_handle);
            _handle = IntPtr.Zero;
        }
    }
}
