import { enableResize, enableReorder } from "../util/table";

export class ModList extends Element {
  this(props) {
    this.props = props;
    this.selectedIndicies = [];
    this.handleRowClick = this.handleRowClick.bind(this);
    this.handleDragStart = this.handleDragStart.bind(this);
    this.updateColumns = this.updateColumns.bind(this);
    this.columns = false;
    this.resizingHeader = false;
    this.isDragging = false;
    this.draggingRow = false;
    this.shadowList = [];
    this.dragPlaceholder = null;
    this.coords = { x: 0, y: 0 };
    this.table = false;
  }

  componentDidMount() {
    this.table = document.querySelector("#ModList");
    this.updateColumns();
    document.body.on("mouseup", this.handleMouseUp.bind(this));
    document.body.on("mousemove", this.handleMouseMove.bind(this));
  }

  updateColumns() {
    if (!this.table) {
      this.table = document.querySelector("#ModList");
    }
    this.columns = {};
    for (const [i, col] of this.table.querySelectorAll("th").entries()) {
      this.columns[i] = { header: col, size: false };
    }
  }

  componentDidUpdate() {
    if (!this.columns) {
      this.updateColumns();
    }
  }

  ["on mousedown at .resize-handle"](e, header) {
    if (!this.isDragging && !this.resizingHeader) {
      if (!this.columns) {
        this.updateColumns();
      }
      this.resizingHeader = header.parentNode;
      header.classList.add("header--being-resized");
    }
  }

  ["on dblclick at .resize-handle"](e, header) {
    header.parentNode.style.maxWidth = "min-content";
    header.parentNode.style.minWidth = undefined;
    header.parentNode.style.width = undefined;
  }

  handleMouseMove(e) {
    if (this.resizingHeader) {
      requestAnimationFrame(() => {
        const horizontalScrollOffset = document.documentElement.scrollLeft;
        const width =
          horizontalScrollOffset +
          e.clientX -
          this.resizingHeader.offsetLeft;
        this.resizingHeader.style.minWidth = Math.max(16, width) + "dip";
        Object.entries(this.columns).forEach(([i, column]) => {
          if (!column.header.style.width) {
            column.header.style.width = parseInt(column.header.clientWidth) + "dip";
          }
        });
      });
    } else if (this.isDragging) {
      return;
    }
  }

  handleMouseUp(e) {
    if (this.resizingHeader) {
      this.resizingHeader.classList.remove("header--being-resized");
      this.resizingHeader = null;
    }
  }

  handleDragStart(evt, index) {
    let { clientX: x, clientY: y } = evt;
    let lasttarget = null;
    let element = evt.target.parentNode;
    if (!element.attributes["class"].includes("selected")) {
      element.classList.add("selected");
      if (evt.ctrlKey) {
        this.selectedIndicies.push(index);
      } else {
        this.selectedIndicies = [index];
      }
    }
    let selectedIndicies = this.selectedIndicies.slice().sort();

    let onmove = (evt) => { lasttarget = evt.target; };

    document.post(() => {
      let image = new Graphics.Image(element);
      document.style.setCursor(image, x, 0);
      for (let el of document.querySelectorAll("tr.selected")) {
        el.style.visibility = "hidden";
      }

      document.state.capture(true);
      document.attributes["dnd"] = "";
      document.on("mousemove", onmove);

      let r = Window.this.doEvent("untilMouseUp");
      document.state.capture(false);
      document.off(onmove);
      document.style.setCursor(null);
      for (let el of document.querySelectorAll("tr.selected")) {
        el.style.visibility = undefined;
      }
      document.attributes["dnd"] = undefined;

      if (r && lasttarget) {
        const parent = lasttarget.parentNode;
        if (parent && parent.nodeName == "tr") {
          const newIdx = parent.attributes["index"] || 0;
          this.props.onReorder(selectedIndicies, newIdx);
          this.selectedIndicies = [...Array(this.selectedIndicies).keys()].map(i => i + newIdx);
          this.componentUpdate({ selectedIndicies: this.selectedIndicies  });
        }
      }
    });
  }


  handleRowClick(e, index) {
    if (!e.ctrlKey) {
      this.componentUpdate({ selectedIndicies: [index] });
      return;
    }
    if (!this.selectedIndicies.includes(index)) {
      this.componentUpdate({ selectedIndicies: [index, ...this.selectedIndicies] });
    } else {
      this.componentUpdate({
        selectedIndicies: this.selectedIndicies.filter(h => h != index)
      });
    }
  }

  render() {
    return (
      <div styleset={__DIR__ + "ModList.css#ModList"}>
        <table #ModList >
          <thead>
            <tr>
              <th>
                {" "}
                <span class="resize-handle" style="display: none;"></span>
              </th>
              <th>
                Mod Name <span class="resize-handle"></span>
              </th>
              <th>
                Author <span class="resize-handle"></span>
              </th>
              <th>
                Version <span class="resize-handle"></span>
              </th>
              <th>
                Priority <span class="resize-handle"></span>
              </th>
            </tr>
          </thead>
          <tbody>

            {this.props.mods.map((mod, i) => (
              <tr
                index={i}
                key={mod.hash}
                onClick={e => this.handleRowClick(e, i)}
                onMouseDragRequest={e => this.handleDragStart(e, i)}
                class={
                  (this.selectedIndicies.includes(i) && "selected") +
                  " " +
                  (!mod.enabled && "disabled")
                }>
                <td>
                  <input
                    type="checkbox"
                    checked={mod.enabled}
                    onClick={() => this.props.onToggle(mod)}
                  />
                </td>
                <td>{mod.meta.name}</td>
                <td>{mod.meta.author}</td>
                <td class="numeric">{mod.meta.version.toFixed(1)}</td>
                <td class="numeric">{i + 1}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    );
  }
}
