// The bar above the keyboard stays hidden (keel-ios KeyboardBar.swift). With it,
// the keyboard takes about 68pt more of the screen on an iPhone 17 (396pt to 328pt).
document.querySelector('.add-task-row.view-add')?.click();
await sleep(1500);
const covered = innerHeight > visualViewport.height ? innerHeight - visualViewport.height : document.documentElement.clientHeight - visualViewport.height;
check(`keyboard covers ${Math.round(covered)}pt; bar hidden ${covered > 0 && covered < 360}`);
