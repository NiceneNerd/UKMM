import * as sys from "@sys";
import * as env from "@env";
import { Back } from "../../icons/Back";
import { Up } from "../../icons/Up";
import { Open } from "../../icons/Open";

const EXTENSIONS = ["zip", "7z"];
const SEP = env.OS.includes("Windows") ? "\\" : "/";

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
    this.folder = env.path("downloads");
    this.goUp = this.goUp.bind(this);
    this.goBack = this.goBack.bind(this);
    this.handleOpen = this.handleOpen.bind(this);
  }

  this(props) {
    this.props = props;
    this.loadFolder();
  }

  goUp() {
    let parts = this.folder.split(SEP);
    if (parts.length == 1) return;
    parts.pop();
    this.componentUpdate({
      history: [...this.history, this.folder],
      folder: parts.join(SEP),
    });
    this.loadFolder();
  }

  goBack() {
    let lastFolder = this.history.pop();
    if (lastFolder) {
      this.componentUpdate({ folder: lastFolder });
      this.loadFolder();
    }
  }

  handleOpen() {
    const file = Window.this.selectFile({
      mode: "open",
      filter:
        "Graphic Pack or RomFS (*.zip, *.7z, rules.txt)|*.zip;*.7z;rules.txt|All Files (*.*)|*.*",
      caption: "Select Mod",
      path: this.folder,
    });
    if (file) {
      this.props.onSelect(file.replace("file:///", ""));
    }
  }

  loadFolder() {
    const items = sys.fs
      .$readdir(this.folder)
      .filter(
        (item) =>
          !item.name.startsWith(".") &&
          (item.type == 2 ||
            EXTENSIONS.includes(item.name.split(".").slice(-1)[0].toLowerCase()))
      )
      .sort((a, b) => {
        if (a.type != b.type) {
          return b.type - a.type;
        } else {
          return a.name > b.name;
        }
      });
    this.componentUpdate({
      children: items.map((item) => ({
        type: item.type == 1 ? "file" : "folder",
        path: this.folder + SEP + item.name,
        name: item.name,
      })),
    });
  }

  ["on dblclick at option.folder"](e, opt) {
    this.componentUpdate({
      history: [...this.history, this.folder],
      folder: opt.value,
    });
    this.loadFolder();
  }

  ["on dblclick at option.file"](e, opt) {
    this.props.onSelect(opt.value);
  }

  ["on keyup at input.path"](e, input) {
    if (e.code == "Enter") {
      const value = e.target.value.replace(new RegExp(`${SEP}$`), "");
      try {
        sys.fs.$stat(value);
        this.componentUpdate({
          history: [...this.history, this.folder],
          folder: value,
        });
        this.loadFolder();
      } catch (error) {
        return;
      }
    }
  }

  ["on keyup at .content"](e, content) {
    if (e.code == "Enter") {
      const selected = this.children.find((item) => item.path == e.target.value);
      if (selected.type == "folder") {
        this.componentUpdate({
          history: [...this.history, this.folder],
          folder: selected.path,
        });
        this.loadFolder();
      } else {
        this.props.onSelect(opt.value);
      }
    } else if (e.code == "ArrowUp" && e.altKey) {
      this.goUp();
    } else if (e.code == "Backspace") {
      this.goBack();
    }
  }

  render() {
    return (
      <div styleset={__DIR__ + "FolderView.css#folder-view"}>
        <nav>
          <button
            class="icon"
            title="Back"
            disabled={this.history.length == 0}
            onClick={this.goBack}
          >
            <Back />
          </button>
          <button class="icon" title="Up" onClick={this.goUp}>
            <Up />
          </button>
          <button class="icon" title="Open…" onClick={this.handleOpen}>
            <Open />
          </button>
          <input type="text" class="path" value={this.folder} />
        </nav>
        <select type="list" class="content" system={this.system}>
          {this.children.map((child) => (
            <option
              class={child.type}
              key={child.path}
              value={child.path}
              title={child.path}
            >
              {child.name}
            </option>
          ))}
        </select>
      </div>
    );
  }
}
