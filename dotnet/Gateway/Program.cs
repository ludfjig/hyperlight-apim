using PolicyEngine;

// Repo root, used to locate the built guest components. Set by the justfile.
var root = Environment.GetEnvironmentVariable("POLICY_DEMO_ROOT")
    ?? Directory.GetCurrentDirectory();

// Customer id -> guest component path.
var guestPaths = new Dictionary<string, string>
{
    ["A"] = Path.Combine(root, "guests/auth_check/auth_check.aot"),
    ["B"] = Path.Combine(root, "guests/path_block/path_block.aot"),
};

// Load one sandbox-backed policy per customer at startup.
var policies = new Dictionary<string, Policy>();
foreach (var (id, path) in guestPaths)
    policies[id] = Policy.Load(File.ReadAllBytes(path));

var builder = WebApplication.CreateBuilder(args);
var app = builder.Build();

app.MapFallback(async ctx =>
{
    var customer = ctx.Request.Headers["X-Customer"].FirstOrDefault();
    if (customer is null || !policies.TryGetValue(customer, out var policy))
    {
        ctx.Response.StatusCode = StatusCodes.Status400BadRequest;
        await ctx.Response.WriteAsync("missing or unknown X-Customer header");
        return;
    }

    var headers = ctx.Request.Headers
        .Select(h => new PolicyHeader(h.Key, h.Value.ToString()))
        .ToList();
    var request = new PolicyRequest(ctx.Request.Method, ctx.Request.Path, headers);

    Decision decision;
    try
    {
        decision = policy.OnRequest(request);
    }
    catch (PolicyException)
    {
        ctx.Response.StatusCode = StatusCodes.Status500InternalServerError;
        await ctx.Response.WriteAsync("policy crashed, contained by the VM");
        return;
    }

    if (decision.Allow)
    {
        ctx.Response.StatusCode = StatusCodes.Status200OK;
        await ctx.Response.WriteAsync($"backend response for {ctx.Request.Path}");
    }
    else
    {
        ctx.Response.StatusCode = decision.Status;
        await ctx.Response.WriteAsync(decision.Message ?? "rejected");
    }
});

app.Run("http://localhost:5000");
