// Shared by both pages. The page draws its own window controls.
function on(id, handler) {
  const el = document.getElementById(id);
  if (el) el.addEventListener("click", handler);
}
on("close", () => VPP.window.close());
on("minimize", () => VPP.window.minimize());
on("maximize", () => VPP.window.maximize());

const button = document.getElementById("hello");
const reset = document.getElementById("reset");
const counter = document.getElementById("count-value");
let clicks = 0;

function render() {
  button.textContent = clicks === 0 ? "Click me" : "Clicked " + clicks;
  counter.textContent = String(clicks);
}

if (button && reset && counter) {
  button.addEventListener("click", () => {
    clicks += 1;
    render();
    VPP.window.popup("Hello from JavaScript! Click #" + clicks);
  });
  reset.addEventListener("click", () => {
    clicks = 0;
    render();
  });
}
console.log("app-window ready");
