import { redirect } from "@sveltejs/kit";
const PUBLIC_PATHS = ["/signin", "/verify"];
const load = ({ locals, url }) => {
  const isPublic = url.pathname === "/" || url.pathname === "/tour" || PUBLIC_PATHS.some((path) => url.pathname.startsWith(path));
  if (!isPublic && locals.sessionId === null) {
    redirect(303, "/signin");
  }
  return { signedIn: locals.sessionId !== null };
};
export {
  load
};
