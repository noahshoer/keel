// How smoothly the title moves while a flick plays out, frame by frame, as drawn.
// Flick a short list (Today) up, from outside, after "FLICK short", then a long
// one down after "FLICK long". The lower the worst jump and the longest stall,
// the smoother. A stall is the title holding still partway between its ends.
const title = () => document.querySelector('.view-title');
const watch = (ms) => new Promise((done) => {
  const seen = [];
  const end = performance.now() + ms;
  const frame = (now) => {
    const r = title().getBoundingClientRect();
    seen.push([now, r.top, document.querySelector('.list-scroll').scrollTop]);
    if (now < end) requestAnimationFrame(frame);
    else {
      const moving = seen.filter((s, i) => i > 0 && Math.abs(s[1] - seen[i - 1][1]) > 0.001);
      if (moving.length < 2) return done({ moved: false });
      const a = seen.indexOf(moving[0]) - 1;
      const b = seen.indexOf(moving[moving.length - 1]);
      const tops = seen.map((s) => s[1]);
      const low = Math.min(...tops), high = Math.max(...tops);
      let jump = 0, stall = 0, startOfHold = seen[a][0];
      for (let i = a + 1; i <= b; i++) {
        const d = Math.abs(seen[i][1] - seen[i - 1][1]);
        jump = Math.max(jump, d);
        // Sitting at either end is the title at rest, not a stall.
        const atEnd = seen[i][1] - low < 0.05 || high - seen[i][1] < 0.05;
        if (d > 0.001 || atEnd) { if (!atEnd) stall = Math.max(stall, seen[i][0] - startOfHold); startOfHold = seen[i][0]; }
      }
      const tail = seen.slice(Math.max(a, b - 14), b + 1).map((s) => `${s[1].toFixed(1)}@${s[2]}`);
      done({ moved: true, frames: b - a, msPerFrame: +((seen[b][0] - seen[a][0]) / (b - a)).toFixed(1), worstJumpPx: +jump.toFixed(2), longestStallMs: Math.round(stall), restsAt: +tops[tops.length - 1].toFixed(2), tail });
    }
  };
  requestAnimationFrame(frame);
});
check('FLICK short');
check('short: ' + JSON.stringify(await watch(8000)));
document.querySelector('.menu-btn').click();
await sleep(800);
[...document.querySelectorAll('.nav-item')].find((n) => n.textContent.includes('Long list')).click();
await sleep(1000);
document.querySelector('.list-scroll').scrollTop = 300;
await sleep(300);
check('FLICK long');
check('long: ' + JSON.stringify(await watch(8000)));
