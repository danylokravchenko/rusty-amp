// Mobile nav toggle + active-link highlighting for rusty-amp docs.
(function () {
  var burger = document.querySelector(".nav__burger");
  var links = document.querySelector(".nav__links");
  if (burger && links) {
    burger.addEventListener("click", function () {
      links.classList.toggle("open");
    });
  }

  // Highlight the current page in the nav.
  var here = location.pathname.split("/").pop() || "index.html";
  document.querySelectorAll(".nav__links a").forEach(function (a) {
    var href = a.getAttribute("href");
    if (!href) return;
    if (href === here || (here === "" && href === "index.html")) {
      a.classList.add("is-active");
    }
  });
})();
