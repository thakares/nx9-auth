const fs = require('fs');
const file = 'ui/dist/assets/boot.js';
let content = fs.readFileSync(file, 'utf8');
content = `
const originalError = console.error;
console.error = function(...args) {
    originalError.apply(console, args);
    const el = document.getElementById("main");
    if (el) {
        el.innerHTML = "<pre style='color:red'>" + args.map(a => String(a)).join(" ") + "</pre>";
    }
};
` + content;
fs.writeFileSync(file, content);
