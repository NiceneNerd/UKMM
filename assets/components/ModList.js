export class ModList extends Element {
  this(props) {
    this.props = props;
    this.selectedIndicies = [];
    this.handleRowClick = this.handleRowClick.bind(this);
    this.handleDragStart = this.handleDragStart.bind(this);
    this.updateColumns = this.updateColumns.bind(this);
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

  handleDragStart(evt, index) {
    let { clientX: x, clientY: y } = evt;
    let lasttarget = null;
    let element = evt.target.closest("tr");
    if (!element.attributes["class"].includes("selected")) {
      element.classList.add("selected");
      if (evt.ctrlKey) {
        this.selectedIndicies.push(index);
      } else {
        for (const idx of this.selectedIndicies) {
          element.parentNode.children[idx].classList.remove("selected");
        }
        this.selectedIndicies = [index];
      }
    }
    let selectedIndicies = this.selectedIndicies.slice().sort();

    let onmove = (evt) => { lasttarget = evt.target; };

    document.post(() => {
      let image = new Graphics.Image(element);
      document.style.setCursor(image, x, 0);
      for (let el of document.querySelectorAll("tr.selected")) {
        el.classList.add("disabled");
        el.classList.remove("selected");
      }

      document.state.capture(true);
      document.attributes["dnd"] = "";
      document.on("mousemove", onmove);

      let r = Window.this.doEvent("untilMouseUp");
      document.state.capture(false);
      document.off(onmove);
      document.style.setCursor(null);
      for (let el of document.querySelectorAll("tr.selected")) {
        el.classList.remove("disabled");
        el.classList.add("selected");
      }
      document.attributes["dnd"] = undefined;

      if (r && lasttarget) {
        // const parent = lasttarget.parentNode;
        const parent = lasttarget.closest("tr");
        if (parent) {
          const newIdx = parent.attributes["index"] || 0;
          this.props.onReorder(selectedIndicies, newIdx);
          this.selectedIndicies = [...Array(this.selectedIndicies).keys()].map(i => i + newIdx);
          this.componentUpdate({ selectedIndicies: this.selectedIndicies });
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
              <th class="checkbox">
                {" "}
              </th>
              <th>{/* <th class="ellipsis"> */}
                Mod Name
              </th>
              <th>
                Author
              </th>
              <th class="numeric">
                Version
              </th>
              <th class="numeric">
                Priority
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
                <td class="checkbox">
                  <input
                    type="checkbox"
                    checked={mod.enabled}
                    onClick={() => this.props.onToggle(mod)}
                  />
                </td>
                <td class="longer">{mod.meta.name}</td>
                <td class="medium">{mod.meta.author}</td>
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
