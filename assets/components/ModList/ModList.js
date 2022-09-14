export class ModList extends Element {
  constructor(props) {
    super(props);
    this.selectedIndicies = [];
    this.handleRowClick = this.handleRowClick.bind(this);
    this.handleDragStart = this.handleDragStart.bind(this);
    this.handleSort = this.handleSort.bind(this);
    this.modGetter = this.modGetter.bind(this);
    this.sort = ["priority", false];
    this.isDragging = false;
    this.draggingRow = false;
    this.shadowList = [];
    this.dragPlaceholder = null;
    this.coords = { x: 0, y: 0 };
    this.table = false;
    this.filtered = [];
  }

  this(props) {
    this.props = props;
    this.filtered = [...props.mods];
  }

  componentDidMount() {
    this.table = document.querySelector("#ModList");
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

  ["on keyup at #ModList"](e) {
    if (this.selectedIndicies.length == 0) return;
    let selectedIdx = this.selectedIndicies[this.selectedIndicies.length - 1];
    let newIdx;
    if (e.code == "ArrowUp") {
      newIdx = Math.max(0, selectedIdx - 1);
    } else if (e.code == "ArrowDown") {
      newIdx = Math.min(this.props.mods.length - 1, selectedIdx + 1);
    } else {
      return;
    }
    this.componentUpdate({ selectedIndicies: [newIdx] });
    this.props.onSelect(newIdx);
    e.preventDefault();
    return false;
  }


  handleRowClick(e, index) {
    if (!e.ctrlKey) {
      this.props.onSelect(index);
      this.componentUpdate({ selectedIndicies: [index] });
    } else if (!this.selectedIndicies.includes(index)) {
      this.componentUpdate({ selectedIndicies: [index, ...this.selectedIndicies] });
    } else {
      this.componentUpdate({
        selectedIndicies: this.selectedIndicies.filter(h => h != index)
      });
    }
  }

  modGetter(field) {
    switch(field) {
      case "enabled":
        return mod => mod.enabled;
      case "priority":
        return mod => this.props.mods.indexOf(mod);
      default:
        return mod => mod.meta[field];
    }
  }

  handleSort(e) {
    const field = e.target.attributes["field"];
    let sort = this.sort;
    if (sort[0] == field) {
      sort[1] = !sort[1];
    } else {
      sort[0] = field;
    }
    const get = this.modGetter(field);
    let filtered = this.filtered.slice();
    filtered.sort((a, b) => {
      let [valA, valB] = [get(a), get(b)];
      if (valA > valB) {
        return sort[1] ? -1 : 1;
      } else if (valA < valB) {
        return sort[1] ? 1 : -1;
      } else {
        return 0;
      }
    });
    this.componentUpdate({ filtered, sort });
  }

  render() {
    return (
      <div styleset={__DIR__ + "ModList.css#ModList"}>
        <table #ModList >
          <thead>
            <tr>
              <th class="checkbox" field="enabled" onClick={this.handleSort}>
                {" "}
              </th>
              <th field="name" onClick={this.handleSort}>
                Mod Name
              </th>
              <th field="category" onClick={this.handleSort}>
                Category
              </th>
              <th class="numeric" field="version" onClick={this.handleSort}>
                Version
              </th>
              <th class="numeric" field="priority" onClick={this.handleSort}>
                Priority
              </th>
            </tr>
          </thead>
          <tbody>
            {this.filtered.map(mod => {
              let i = this.props.mods.indexOf(mod);
              return (
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
                  <td class="medium">{mod.meta.category}</td>
                  <td class="numeric">{mod.meta.version.toFixed(1)}</td>
                  <td class="numeric">{i}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    );
  }
}
