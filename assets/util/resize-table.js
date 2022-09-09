export function enableTableResize(table) {
  // Code By Webdevtrick ( https://webdevtrick.com )
  const min = 1.5;
  // The max (fr) values for grid-template-columns
  const columnTypeToRatioMap = {
    numeric: 1.5,
    "text-short": 20,
    "text-long": 100,
  };

  const columns = [];
  let headerBeingResized;

  // The next three functions are mouse event callbacks

  // Where the magic happens. I.e. when they're actually resizing
  const onMouseMove = (e) =>
    requestAnimationFrame(() => {
      // Calculate the desired width
      const horizontalScrollOffset = document.documentElement.scrollLeft;
      const width = horizontalScrollOffset + e.clientX - headerBeingResized.offsetLeft;

      // Update the column object with the new size value
      const column = columns.find(({ header }) => header === headerBeingResized);
      console.log(column);
      column.header.style.minWidth = Math.max(min, width) + "px";

      // For the other headers which don't have a set width, fix it to their computed width
      columns.forEach((column) => {
          // isn't fixed yet (it would be a pixel value otherwise)
          column.header.style.minWidth = parseInt(column.header.clientWidth, 10) + "px";
      });
    });

  // Clean up event listeners, classes, etc.
  const onMouseUp = () => {
    console.log("onMouseUp");

    document.body.removeEventListener("mousemove", onMouseMove);
    document.body.removeEventListener("mouseup", onMouseUp);
    headerBeingResized.classList.remove("header--being-resized");
    headerBeingResized = null;
  };

  // Get ready, they're about to resize
  const initResize = e => {
    headerBeingResized = e.target.parentNode;
    document.body.on("mousemove", onMouseMove);
    document.body.on("mouseup", onMouseUp);
    headerBeingResized.classList.add("header--being-resized");
  };

  // Let's populate that columns array and add listeners to the resize handles
  table.querySelectorAll("th").forEach((header) => {
    const max = columnTypeToRatioMap[header.attributes["data-type"]] + "em";
    columns.push({
      header,
      // The initial size value for grid-template-columns:
      size: `minmax(${min}em, ${max})`,
    });

    header.querySelector(".resize-handle").on("mousedown", initResize);
  });
}
