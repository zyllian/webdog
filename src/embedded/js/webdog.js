(function () {
  "use strict";

  for (const copyButton of document.querySelectorAll(".wd-codeblock .copy")) {
    const source = copyButton.parentElement.querySelector("pre").innerText;
    copyButton.addEventListener("click", async () => {
      await navigator.clipboard.writeText(source);
    });
    copyButton.style.display = "inline-block";
  }
})();
