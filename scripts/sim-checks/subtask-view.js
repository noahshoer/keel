// A task's view, then one of its sub-tasks' (Back and the parent's name).
document.querySelector('.task-body').click();
await sleep(3000);
document.querySelector('.tv-sub-body').click();
await sleep(2500);
check(`header: ${document.querySelector('.tv-sheet .tv-head').innerText.replace(/\n/g, ' / ')}`);
const glassIds = () => [...document.querySelectorAll('.tv-sheet .tv-head button')].map((b) => `${b.className.split(' ')[0]} ${Math.round(b.getBoundingClientRect().width)}`).join(', ');
check(`header buttons: ${glassIds()}`);
await sleep(1500);
document.querySelector('.tv-sheet .tv-head .tv-back').click();
await sleep(1500);
check(`after Back: ${glassIds()}`);
