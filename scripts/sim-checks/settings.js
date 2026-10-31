// Settings opens from the sidebar's gear and shows this device's values;
// a menu opens over it and Escape closes the menu, then the dialog. Changes
// nothing.
const q = (s) => document.querySelector(s);
const text = (el) => (el ? el.textContent.trim() : '?');
// The gear sits in the drawer on the phone; the dialog draws inside it.
q('.menu-btn').click();
await sleep(1000);
q('.settings-btn').click();
await sleep(1500);
check(`modal: ${!!q('.modal')} sections ${[...document.querySelectorAll('.modal-section')].map(text).join(' / ')}`);
const sw = [...document.querySelectorAll('.modal .switch')].map((b) => `${b.title}=${b.getAttribute('aria-checked')}`);
check(`switches: ${sw.join(', ')}`);
check(`menus: ${[...document.querySelectorAll('.modal .chip-btn')].map(text).join(' / ')}`);
check(`upcoming: ${document.querySelectorAll('.upcoming-list li').length} rows; first ${text(q('.upcoming-list li'))}`);
check(`marks: ${[...document.querySelectorAll('.mark-choice')].map((b) => text(b) + (b.classList.contains('chosen') ? '*' : '')).join(', ')}`);
check(`foot: ${text(q('.modal-foot'))}`);
q('.modal .chip-btn').click();
await sleep(800);
check(`menu open: ${!!q('.modal .popover.menu')} items ${document.querySelectorAll('.modal .popover .menu-item').length}`);
await sleep(1500);
q('.modal').dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
await sleep(500);
check(`after Escape: menu ${!!q('.modal .popover.menu')} modal ${!!q('.modal')}`);
q('.modal').dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
await sleep(500);
check(`after second Escape: modal ${!!q('.modal')}`);
