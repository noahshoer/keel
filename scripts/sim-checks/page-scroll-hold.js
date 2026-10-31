// While the task view is open, Swift holds the page's scroll (keel-ios
// PageScroll), so WebKit revealing a field in the sheet can't move the list
// under it. A headless simulator has no keyboard, so this asks the page to
// scroll with and without the hold; the evidence is the diag log's
// `SV offset` lines (the web view's own scroll, which WebKit's keyboard
// reveal moves): 0,0 while held, 0,200 once free. `window.scrollY` isn't:
// a scroll asked for from the page moves the page's own scroll position
// first, which the hold doesn't touch. The page is made taller than the
// screen so it can scroll at all.
document.documentElement.style.height = '3000px';
document.body.style.height = '3000px';
document.querySelector('.task-body').click();
await sleep(1500);
check('open: asking the page to scroll to 200');
window.scrollTo(0, 200);
await sleep(500);
window.scrollTo(0, 0);
await sleep(300);
document.querySelector('.tv-scrim').click();
await sleep(1500);
check('closed: asking the page to scroll to 200');
window.scrollTo(0, 200);
await sleep(500);
window.scrollTo(0, 0);
document.documentElement.style.height = '';
document.body.style.height = '';
check('done');
