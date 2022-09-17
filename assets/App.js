import { ModList } from "./components/ModList/ModList";
import { Log } from "./components/Log/Log";
import { MenuBar } from "./components/MenuBar/MenuBar";
import { Tabs, Tab } from "./components/Tabs/Tabs";
import { ProfileMenu } from "./components/ProfileMenu/ProfileMenu";
import { ModInfo } from "./components/ModInfo/ModInfo";
import { Toolbar } from "./components/Toolbar/Toolbar";
import { FolderView } from "./components/FolderView/FolderView";
import { DirtyBar } from "./components/DirtyBar/DirtyBar";
import Modal from "./components/Modal";

export class App extends Element {
  dirty = false;
  mods = [];
  currentMod = 0;
  profiles = [];
  currentProfile = "Default";
  log = [];

  constructor(props) {
    super(props);
    this.props = props;
    this.api = this.api.bind(this);
    Window.this.api = this.api;
    this.handleToggle = this.handleToggle.bind(this);
    this.handleReorder = this.handleReorder.bind(this);
    this.handleLog = this.handleLog.bind(this);
    Window.this.log = this.handleLog;
    this.handleSelect = this.handleSelect.bind(this);
    this.handleOpen = this.handleOpen.bind(this);
    this.handleApply = this.handleApply.bind(this);
    this.handleCancel = this.handleCancel.bind(this);
  }

  api(task, ...args) {
    return Window.this.xcall(task, ...args);
  }

  componentDidMount() {
    const mods = this.api("mods");
    const profiles = this.api("profiles");
    const currentProfile = this.api("currentProfile");
    this.componentUpdate({ mods, profiles, currentProfile });
  }

  async doTask(task, ...args) {
    let modal = new Window({
      type: Window.DIALOG_WINDOW,
      parent: Window.this,
      url: __DIR__ + "progress.html",
      alignment: 5,
      parameters: {
        theme: document.getRootNode().getAttribute("theme"),
      },
    });
    try {
      const res = await (() => {
        return new Promise((resolve) => {
          this.api(task, ...args, resolve);
        });
      })();
      modal.close();
      if (res?.error) throw res;
    } catch (error) {
      modal.close();
      console.log(error);
      Window.this.modal(
        <error resizable={true} caption="Error">
          <Modal>{error.error}</Modal>
        </error>
      );
    }
  }

  handleToggle(mod) {
    mod.enabled = !mod.enabled;
    this.componentUpdate({ mods: this.mods.slice(), dirty: true });
  }

  handleReorder(oldIdxs, newIdx) {
    const modsToMove = oldIdxs.map((i) => this.mods[i]);
    for (const mod of modsToMove) {
      this.mods.splice(this.mods.indexOf(mod), 1);
    }
    const mods =
      newIdx == 0
        ? [...modsToMove, ...this.mods]
        : [...this.mods.slice(0, newIdx), ...modsToMove, ...this.mods.slice(newIdx)];
    this.componentUpdate({ mods, dirty: true });
  }

  handleSelect(index) {
    this.componentUpdate({ currentMod: index });
  }

  handleLog(record) {
    let log = this.log;
    log.push(record);
    this.componentUpdate({ log });
  }

  handleOpen(path) {
    console.log(path);
  }

  async handleApply() {
    await this.doTask("apply", JSON.stringify(this.mods));
    this.componentUpdate({ mods: this.api("mods"), dirty: false });
  }

  handleCancel() {
    this.componentUpdate({ mods: this.api("mods"), dirty: false });
  }

  render() {
    return (
      <div style="flow: vertical; size: *;">
        <MenuBar />
        <frameset cols="*,36%" style="size: *;">
          <div style="size: *;">
            <Toolbar>
              <ProfileMenu
                currentProfile={this.currentProfile}
                profiles={this.profiles}
              />
              <div class="spacer"></div>
              <div class="counter">
                <strong>{this.mods.length}</strong> Mods /{" "}
                <strong>{this.mods.filter((m) => m.enabled).length} </strong>
                Active
              </div>
            </Toolbar>
            <frameset rows="*,15%" style="size: *;">
              <div class="flow: vertical; size: *;">
                <ModList
                  mods={this.mods}
                  onToggle={this.handleToggle}
                  onReorder={this.handleReorder}
                  onSelect={this.handleSelect}
                />
                {this.dirty ? <DirtyBar onApply={this.handleApply} onCancel={this.handleCancel} /> : []}
              </div>
              <splitter />
              <Log logs={this.log} />
            </frameset>
          </div>
          <splitter />
          <Tabs>
            <Tab label="Mod Info">
              <ModInfo mod={this.mods[this.currentMod]} />
            </Tab>
            <Tab label="Install">
              <FolderView onSelect={this.handleOpen} />
            </Tab>
          </Tabs>
        </frameset>
      </div>
    );
  }
}
