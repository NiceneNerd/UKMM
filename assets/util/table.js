let isDraggingStarted = false;
let headerBeingResized;

export function enableResize(table) {
    const min = 16;

    const columns = [];

    const onMouseMove = e => {
        if (isDraggingStarted) return;
        requestAnimationFrame(() => {
            // Calculate the desired width
            const horizontalScrollOffset = document.documentElement.scrollLeft;
            const width =
                horizontalScrollOffset +
                e.clientX -
                headerBeingResized.offsetLeft;

            const column = columns.find(
                ({ header }) => header === headerBeingResized
            );
            column.header.style.minWidth = Math.max(min, width) + "dip";

            columns.forEach(column => {
                if (!column.header.style.width) {
                    column.header.style.width =
                        parseInt(column.header.clientWidth) + "dip";
                }
            });
        });
    };

    const onMouseUp = () => {
        if (isDraggingStarted) return;
        document.body.removeEventListener("mousemove", onMouseMove);
        document.body.removeEventListener("mouseup", onMouseUp);
        headerBeingResized.classList.remove("header--being-resized");
        headerBeingResized = null;
    };

    const initResize = e => {
        if (isDraggingStarted) return;
        headerBeingResized = e.target.parentNode;
        document.body.on("mousemove", onMouseMove);
        document.body.on("mouseup", onMouseUp);
        headerBeingResized.classList.add("header--being-resized");
    };

    table.querySelectorAll("th").forEach(header => {
        columns.push({
            header
        });
        header.querySelector(".resize-handle").on("mousedown", initResize);
    });
}

/// Taken from https://htmldom.dev/drag-and-drop-table-row
export function enableReorder(table) {
    let draggingEle;
    let draggingRowIndex;
    let list;
    let placeholder;
    let x = 0,
        y = 0;

    const swap = function (nodeA, nodeB) {
        const parentA = nodeA.parentNode;
        const siblingA =
            nodeA.nextSibling === nodeB ? nodeA : nodeA.nextSibling;

        nodeB.parentNode.insertBefore(nodeA, nodeB);
        parentA.insertBefore(nodeB, siblingA);
    };

    const isAbove = function (nodeA, nodeB) {
        const rectA = nodeA.getBoundingClientRect();
        const rectB = nodeB.getBoundingClientRect();

        return rectA.top + rectA.height / 2 < rectB.top + rectB.height / 2;
    };

    const mouseDownHandler = e => {
        if (headerBeingResized) return;
        const originalRow = e.target.parentNode;
        draggingRowIndex = [].slice
            .call(table.querySelectorAll("tr"))
            .indexOf(originalRow);
        x = e.clientX;
        y = e.clientY;
        document.body.on("mousemove", mouseMoveHandler);
        document.body.on("mouseup", mouseUpHandler);
    };

    const cloneTable = () => {
        if (headerBeingResized) return;
        const rect = table.getBoundingClientRect();
        const width = parseInt(table.clientWidth);
        list = document.createElement("div");
        list.style.position = "absolute";
        list.style.left = `${rect.left}px`;
        list.style.top = `${rect.top}px`;
        table.parentNode.insertBefore(list, table);
        table.style.visibility = "hidden";
        table.querySelectorAll("tr").forEach(row => {
            const item = document.createElement("div");
            const newTable = document.createElement("table");
            newTable.setAttribute("class", "clone-table");
            newTable.style.width = `${width}px`;
            const newRow = document.createElement("tr");
            const cells = [].slice.call(row.children);
            cells.forEach(cell => {
                const newCell = cell.cloneNode(true);
                newCell.style.width = `${parseInt(
                    window.getComputedStyle(cell).width
                )}px`;
                newRow.appendChild(newCell);
            });
            newTable.appendChild(newRow);
            item.appendChild(newTable);
            list.appendChild(item);
        });
    };

    table.querySelectorAll("tr").forEach((row, index) => {
        if (index >= 1) row.on("mousedown", mouseDownHandler);
    });

    const mouseMoveHandler = e => {
        if (headerBeingResized) return;
        if (!isDraggingStarted) {
            isDraggingStarted = true;
            cloneTable();
            draggingEle = list.children[draggingRowIndex];

            placeholder = document.createElement("div");
            placeholder.classList.add("placeholder");
            draggingEle.parentNode.insertBefore(
                placeholder,
                draggingEle.nextSibling
            );
            placeholder.style.height = `${draggingEle.offsetHeight}px`;
        }

        draggingEle.style.position = "absolute";
        draggingEle.style.top = `${draggingEle.offsetTop + e.clientY - y}px`;
        draggingEle.style.left = `${draggingEle.offsetLeft + e.clientX - x}px`;

        x = e.clientX;
        y = e.clientY;

        const prevEle = draggingEle.previousElementSibling;
        const nextEle = placeholder.nextElementSibling;

        if (
            prevEle &&
            prevEle.previousElementSibling &&
            isAbove(draggingEle, prevEle)
        ) {
            swap(placeholder, draggingEle);
            swap(placeholder, prevEle);
            return;
        }

        if (nextEle && isAbove(nextEle, draggingEle)) {
            swap(nextEle, placeholder);
            swap(nextEle, draggingEle);
        }
    };

    const mouseUpHandler = () => {
        if (headerBeingResized) return;
        placeholder && placeholder.parentNode.removeChild(placeholder);
        draggingEle.classList.remove("dragging");
        draggingEle.style.removeProperty("top");
        draggingEle.style.removeProperty("left");
        draggingEle.style.removeProperty("position");
        const endRowIndex = Array.from(list.children).indexOf(draggingEle);
        isDraggingStarted = false;
        list.parentNode.removeChild(list);
        let rows = table.querySelectorAll("tr").slice();
        draggingRowIndex > endRowIndex
            ? rows[endRowIndex].parentNode.insertBefore(
                  rows[draggingRowIndex],
                  rows[endRowIndex]
              )
            : rows[endRowIndex].parentNode.insertBefore(
                  rows[draggingRowIndex],
                  rows[endRowIndex].nextSibling
              );
        table.style.removeProperty("visibility");
        document.body.removeEventListener("mousemove", mouseMoveHandler);
        document.body.removeEventListener("mouseup", mouseUpHandler);
    };
}
