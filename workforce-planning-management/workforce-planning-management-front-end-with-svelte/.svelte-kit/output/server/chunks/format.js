function percentOf(done, total, fallback = "—") {
  if (total === 0) return fallback;
  return `${Math.round(done / total * 100)}%`;
}
function mean(value) {
  if (value === null || value === void 0) return null;
  return value.toFixed(1);
}
export {
  mean as m,
  percentOf as p
};
