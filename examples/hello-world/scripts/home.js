// Home page only.
const button = document.getElementById("hello");
const reset = document.getElementById("reset");
const counter = document.getElementById("count-value");
let clicks = 0;

function render() {
  button.textContent = clicks === 0 ? "Click me" : "Clicked " + clicks;
  counter.textContent = String(clicks);
}

button.addEventListener("click", (event) => {
  clicks += 1;
  render();
  console.log("clicked", event.target.id, "count", clicks);
  VPP.window.popup("Hello from JavaScript! Click #" + clicks);
});

reset.addEventListener("click", () => {
  clicks = 0;
  render();
});
