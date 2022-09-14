import * as sys from "@sys";
import * as env from "@env";

export class FolderView extends Element {
  path = "";
  filter = null; // file filter
  currentNode = null; // like "foo.txt" or ".."
  elcontent = null; // dom element
  elpath = null; // dom element

  constructor(props) {
    super();
    this.path = props?.path || env.path("USER_DOCUMENTS") + "/";
  }

  componentDidMount() {
    const pathAttr = this.attributes["path"];
    const filterAttr = this.attributes["filter"];
    if (filterAttr !== undefined) this.filter = filterAttr.split(";");
    if (pathAttr !== this.path)
      this.navigateTo(this.path || env.path("USER_DOCUMENTS") + "/");
  }

  activateCurrent() {
    const [type, name, path] = this.current;
    if (type == "folder") {
      if (name == "..") {
        const [parent, child] = sys.fs.splitpath(this.path);
        [parent, child] = sys.fs.splitpath(parent);
        path = parent + "/";
        this.navigateTo(path, child);
      } else {
        this.navigateTo(path, "..");
      }
      this.post(new Event("folder-change", { data: path, bubbles: true }));
    } else {
      this.post(new Event("file-activate", { data: path, bubbles: true }));
    }
    return true;
  }

  navigateTo(path, currentNode = "..") {
    if (sys.fs.match(path, "file://*")) path = URL.toPath(path);

    if (!path.endsWith("/")) path += "/";

    try {
      let list = sys.fs.$readdir(path); // note: to speed up things I use sync version of readdir
      let files = [];
      let folders = [];
      let filter = this.filter;
      for (const entry of list) {
        if (sys.fs.match(entry.name, ".*") || sys.fs.match(entry.name, "~*")) continue; // these are "hidden" files
        if (entry.type == 2) folders.push(entry.name);
        else if (filter) {
          for (let f of filter)
            if (sys.fs.match(entry.name, f)) {
              files.push(entry.name);
              break;
            }
        } else files.push(entry.name);
      }

      folders.sort();
      files.sort();

      this.componentUpdate({
        path: path,
        files: files,
        folders: folders,
        currentNode: currentNode,
      });
    } catch (e) {
      console.error(e.toString());
    }
  }

  fullPath(localName) {
    return this.path + localName;
  }

  get current() {
    const option = this.contentPane.$("option:current");
    if (option)
      return [
        option.classList.contains("folder") ? "folder" : "file",
        option.innerText,
        option.attributes["filename"],
      ];
    return null;
  }

  get parentPath() {
    let [parent, child] = sys.fs.splitpath(this.path);
    [parent, child] = sys.fs.splitpath(parent);
    return [parent + "/", child].join("");
  }

  ["on dblclick at select.content>option"]() {
    this.activateCurrent();
  }

  ["on change"]() {
    const option = this.contentPane.$("option:current");
    if (option) this.currentNode = option.text;
    else this.currentNode = null;
  }

  ["on keyup at select.content"](evt, content) {
    switch (evt.code) {
      case "Escape":
        this.navigateTo(this.parentPath[0]);
        return true;
      case "Enter":
        this.activateCurrent();
        return true;
    }
  }

  ["on keydown at select.content"](evt) {
    switch (evt.code) {
      case "ArrowUp": {
        //not handled by select.content - on very first item
        const path = this.pathPane;
        path.$("option:first-child").click();
        path.focus();
        return true;
      }
    }
  }

  ["on ^keydown at select.path"](evt) {
    switch (evt.code) {
      case "ArrowDown": {
        //not handled by select.content - on very first item
        const content = this.contentPane;
        content.$("option:first-child").click();
        content.focus();
        return true;
      }
    }
  }

  ["on keyup at select.path"](evt, path) {
    switch (evt.code) {
      case "Escape":
        this.navigateTo(this.parentPath[0]);
        return true;
      case "Enter": {
        const current = path.$("option:current");
        if (!current || current.elementIndex == 0) {
          const [path, local] = this.parentPath;
          this.navigateTo(path, local);
        } else {
          const path = current.attributes["filename"];
          const next = current.nextElementSibling;
          const local = next ? next.innerText : null;
          this.navigateTo(path, local);
        }
        this.post(() => this.contentPane.focus());
        return true;
      }
    }
  }

  ["on mouseup at select.path>option"](evt, option) {
    const path = option.attributes["filename"];
    const next = option.nextElementSibling;
    const local = next ? next.innerText : null;
    this.navigateTo(path, local);
    this.post(() => this.contentPane.focus());
    return true;
  }

  get contentPane() {
    if (!this.elcontent) this.elcontent = this.$("select.content");
    return this.elcontent;
  }

  get pathPane() {
    if (!this.elpath) this.elpath = this.$("select.path");
    return this.elpath;
  }

  render() {
    const path = this.path;
    const currentName = this.currentNode;
    let pathparts = path.split("/");
    pathparts.pop();
    function partialpath(i) {
      return pathparts.slice(0, i + 1).join("/");
    }

    const folders = this.folders.map((name) => (
      <option
        class="folder"
        key={name + "/d"}
        filename={path + name}
        state-current={currentName == name}
      >
        {name}
      </option>
    ));
    const files = this.files.map((name) => (
      <option
        class="file"
        key={name + "/f"}
        filename={path + name}
        state-current={currentName == name}
      >
        {name}
      </option>
    ));

    const pathoptions = pathparts.map((name, index) => (
      <option class="folder" filename={partialpath(index)}>
        {name}
      </option>
    ));
    const first =
      this.path && this.path != "/" ? (
        <option
          class="up"
          filename={this.parentPath}
          state-current={currentName == ".."}
        ></option>
      ) : (
        []
      );

    return (
      <folder path={path} styleset="FolderView.css#folder-view">
        <select type="list" class="path">
          {first}
          {pathoptions}
        </select>
        <select type="list" class="content">
          {folders}
          {files}
        </select>
      </folder>
    );
  }
}

globalThis.FolderView = FolderView;
