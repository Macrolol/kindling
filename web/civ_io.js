// Kindling browser save/load plugin: download a save, pick a file to load.
var civ_pending_file = null;
miniquad_add_plugin({
  name: "civ_io", version: 1,
  register_plugin: function (importObject) {
    importObject.env.civ_download = function (p, len, np, nl) {
      var data = new Uint8Array(wasm_memory.buffer, p, len).slice();
      var name = new TextDecoder().decode(new Uint8Array(wasm_memory.buffer, np, nl));
      var a = document.createElement("a");
      a.href = URL.createObjectURL(new Blob([data], { type: "application/octet-stream" }));
      a.download = name; document.body.appendChild(a); a.click();
      setTimeout(function () { URL.revokeObjectURL(a.href); a.remove(); }, 2000);
    };
    importObject.env.civ_pick_file = function () {
      var i = document.createElement("input"); i.type = "file"; i.accept = ".civsave,.json";
      i.onchange = function () { var f = i.files[0]; if (f) f.arrayBuffer().then(function (b) { civ_pending_file = new Uint8Array(b); }); };
      i.click();
    };
    importObject.env.civ_file_len = function () { return civ_pending_file ? civ_pending_file.length : -1; };
    importObject.env.civ_file_take = function (p) {
      new Uint8Array(wasm_memory.buffer, p, civ_pending_file.length).set(civ_pending_file); civ_pending_file = null;
    };
  }
});
// keep Ctrl+S / Ctrl+O for the app instead of the browser
window.addEventListener("keydown", function (e) { if ((e.ctrlKey || e.metaKey) && (e.key === "s" || e.key === "o")) e.preventDefault(); }, true);
