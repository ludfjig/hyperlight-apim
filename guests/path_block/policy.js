export const handler = {
  onRequest(req) {
    if (req.path.startsWith("/admin")) {
      return { tag: "reject", val: { status: 403, message: "admin path blocked" } };
    }
    return { tag: "allow" };
  },
};
