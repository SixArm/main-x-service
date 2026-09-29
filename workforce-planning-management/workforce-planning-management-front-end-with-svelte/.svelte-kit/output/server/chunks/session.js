const SESSION_COOKIE = "__Host-mxi_session";
const SESSION_COOKIE_OPTIONS = {
  path: "/",
  httpOnly: true,
  secure: true,
  sameSite: "lax"
};
function parseSessionId(setCookie) {
  const prefix = `${SESSION_COOKIE}=`;
  const segment = setCookie.split(";").map((s) => s.trim()).find((s) => s.startsWith(prefix));
  if (!segment) return null;
  const value = segment.slice(prefix.length);
  return value.length > 0 ? value : null;
}
function sessionIdFromResponse(response) {
  const headers = response.headers;
  const lines = headers.getSetCookie?.() ?? [
    response.headers.get("set-cookie") ?? ""
  ];
  for (const line of lines) {
    const sid = parseSessionId(line);
    if (sid) return sid;
  }
  return null;
}
export {
  SESSION_COOKIE as S,
  SESSION_COOKIE_OPTIONS as a,
  sessionIdFromResponse as s
};
