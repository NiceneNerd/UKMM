import * as sys from "@sys";
import * as env from "@env";

export class FolderView extends Element {
  folder = "";
  children = [];
  history = [];
  system = "windows";

  constructor(props) {
    super(props);
    if (env.OS.includes("Windows")) {
      this.system = "windows";
    } else {
      this.system = "linux";
    }
    this.folder = env.path("downloads").replace(/\\/g, "/");
    this.goUp = this.goUp.bind(this);
    this.goBack = this.goBack.bind(this);
  }

  this(props) {
    this.props = props;
    this.loadFolder();
  }

  goUp() {
    let parts = this.folder.split("/");
    if (parts.length == 1) return;
    parts.pop();
    this.componentUpdate({ history: [...this.history, this.folder], folder: parts.join("/") });
    this.loadFolder();
  }

  goBack() {
    let lastFolder = this.history.pop();
    if (lastFolder) {
      this.componentUpdate({ folder: lastFolder });
      this.loadFolder();
    }
  }

  loadFolder() {
    const items = sys.fs.$readdir(this.folder);
    this.componentUpdate({
      children: items.map(item => ({ type: item.type == 1 ? "file" : "folder", path: this.folder + "/" + item.name, name: item.name }))
    });
  }

  ["on dblclick at option.folder"](e, opt) {
    this.componentUpdate({ history: [...this.history, this.folder], folder: opt.value });
    this.loadFolder();
  }

  ["on dblclick at option.file"](e, opt) {
    console.log(opt.value);
    this.props.onSelect(opt.value);
  }

  ["on keyup at .content"](e, content) {
    if (e.code == "Enter") {
      const selected = this.children.find(item => item.path == e.target.value);
      console.log("Selected", selected);
      if (selected.type == "folder") {
        this.componentUpdate({ history: [...this.history, this.folder], folder: selected.path });
      } else {
        console.log(selected);
      }
      this.loadFolder();
    } else if (e.code == "ArrowUp" && e.altKey) {
      this.goUp();
      e.preventDefault();
      return;
    } else if (e.code == "Backspace") {
      this.goBack();
    }
  }

  render() {
    return (
      <div styleset={__DIR__ + "FolderView.css#folder-view"}>
        <nav>
          <button disabled={this.history.length == 0} onClick={this.goBack}>Back</button>
          <button onClick={this.goUp}>Up</button>
          <input type="text" value={this.folder} />
        </nav>
        <select|list.content system={this.system}>
        {this.children.map(child => (
          <option class={child.type} key={child.path} value={child.path} title={child.path}>{child.name}</option>
        ))}
      </select>
      </div >
    );
  }
}