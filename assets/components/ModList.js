import { enableTableResize } from "../util/resize-table";
import { VirtualList } from "./VirtualList";

class VirtualTableBody extends VirtualList {
  list; // array of items

  this(props) {
    this.list = props.list;
    this.props = props;
  }

  itemAt(at) {
    return this.list[at];
  }

  totalItems() {
    return this.list.length;
  }

  indexOf(item) {
    return this.list.indexOf(item);
  }

  // overridable
  renderItem(item, isCurrent, isSelected) {
    return (
      <tr key={item.hash}>
        <td>
          <input
            type="checkbox"
            checked={item.enabled}
            onClick={() => this.props.onToggle(item)}
          />
        </td>
        <td>{item.meta.name}</td>
        <td>{item.meta.author}</td>
        <td>{item.meta.version.toFixed(1)}</td>
        <td>{this.indexOf(item) + 1}</td>
      </tr>
    );
  }

  renderList(items) {
    return <tbody styleset={__DIR__ + "VirtualTable.css#tbody"}>{items}</tbody>;
  }
}
export class ModList extends Element {
  this(props) {
    // super(props);
    this.props = props;
  }

  componentDidMount() {
    enableTableResize(document.getElementById("ModList"));
  }

  render() {
    return (
      <table#ModList styleset={__DIR__ + "ModList.css#ModList"}>
        <thead>
          <tr>
            <th data-type="numeric">  <span class="resize-handle"></span></th>
            <th data-type="text-long">Mod Name <span class="resize-handle"></span></th>
            <th data-type="text-short">Author <span class="resize-handle"></span></th>
            <th data-type="numeric">Version <span class="resize-handle"></span></th>
            <th data-type="numeric">Priority <span class="resize-handle"></span></th>
          </tr>
        </thead>
        <VirtualTableBody list={this.props.mods} onToggle={this.props.onToggle} />
      </table>
    );
  }
}
