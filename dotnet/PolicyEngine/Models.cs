namespace PolicyEngine;

/// The request the gateway hands to a policy.
public sealed record PolicyRequest(string Method, string Path, IReadOnlyList<PolicyHeader> Headers);

public sealed record PolicyHeader(string Name, string Value);

/// A policy decision. When `Allow` is false, `Status` and `Message`
/// carry the rejection response.
public sealed class Decision
{
    public required bool Allow { get; init; }
    public ushort Status { get; init; }
    public string? Message { get; init; }
}

/// Raised when the native engine reports an error or a guest crashes.
public sealed class PolicyException : Exception
{
    public PolicyException(string message) : base(message) { }
}

