import { S as SESSION_COOKIE } from "../chunks/session.js";
const handle = async ({ event, resolve }) => {
  event.locals.sessionId = event.cookies.get(SESSION_COOKIE) ?? null;
  return resolve(event);
};
export {
  handle
};
