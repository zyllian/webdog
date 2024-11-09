(function () {
  "use strict";

  console.log("Connecting...");

  let socket;

  function start(reload) {
    socket = new WebSocket("ws://127.0.0.1:8080");
    let reloading = false;
    socket.onmessage = function (ev) {
      if (ev.data === "reload") {
        reloading = true;
        console.log("Reloading...");
        location.reload();
      }
    };
    socket.onclose = function () {
      if (!reloading) {
        console.error("Connection closed.");
        setTimeout(() => {
          console.log("Retrying connection...");
          start(true);
        }, 2000);
      }
    };
    socket.onopen = function () {
      console.log("Connected!");
      if (reload) {
        location.reload();
      }
    };
  }

  start(false);
})();
