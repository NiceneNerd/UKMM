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
import { Busy } from "./components/Busy/Busy";
import { Error } from "./components/Error/Error";

export class App extends Element {
  dirty = false;
  busy = false;
  files = {};
  mods = [];
  currentMod = 0;
  profiles = [];
  currentProfile = "Default";
  log = [];

  this() {
    Window.this.api = this.api;
    Window.this.log = this.handleLog;
  }

  componentDidMount = () => {
    const profiles = this.api("profiles");
    const currentProfile = this.api("currentProfile");
    this.componentUpdate({ profiles, currentProfile });
    this.loadMods();
  };

  api = (task, ...args) => {
    return Window.this.xcall(task, ...args);
  };

  showError(error) {
    console.log("Error: ", error);
    Window.this.modal(
      <error resizable={true} caption="Error">
        <Error error={error} />
      </error>
    );
  }

  loadMods = () => {
    const mods = this.api("mods");
    const files = mods.reduce((files, mod) => {
      for (const file of mod.manifest.aoc.map(f => "DLC Files/" + f)) {
        if (!files.hasOwnProperty(file)) {
          files[file] = [];
        }
        files[file].push(mod);
      }
      for (const file of mod.manifest.content.map(f => "Base Files/" + f)) {
        if (!files.hasOwnProperty(file)) {
          files[file] = [];
        }
        files[file].push(mod);
      }
      return files;
    }, {});
    console.log(files);
    Window.this.files = files;
    this.componentUpdate({ mods });
  };

  doTask = async (task, ...args) => {
    this.componentUpdate({ busy: true });
    const res = await (() => {
      return new Promise(resolve => {
        this.api(task, ...args, resolve);
      });
    })();
    this.componentUpdate({ busy: false });
    if (res?.msg || res?.trace) throw res;
  };

  handleToggle = mod => {
    mod.enabled = !mod.enabled;
    this.componentUpdate({ mods: this.mods.slice(), dirty: true });
  };

  handleReorder = (oldIdxs, newIdx) => {
    const modsToMove = oldIdxs.map(i => this.mods[i]);
    for (const mod of modsToMove) {
      this.mods.splice(this.mods.indexOf(mod), 1);
    }
    const mods =
      newIdx == 0
        ? [...modsToMove, ...this.mods]
        : [
            ...this.mods.slice(0, newIdx),
            ...modsToMove,
            ...this.mods.slice(newIdx)
          ];
    this.componentUpdate({ mods, dirty: true });
  };

  handleSelect = index => {
    this.componentUpdate({ currentMod: index });
  };

  handleLog = record => {
    let log = this.log;
    log.push(record);
    this.componentUpdate({ log });
    if (this.busy) {
      document.body.patch(Window.this.app);
    }
  };

  handleOpen = async path => {
    try {
      const res = await this.doTask("parse_mod", path);
      console.log(res);
    } catch (error) {
      if (error.msg && error.msg == "Mod missing meta file") {
        // try converting
      } else {
        this.showError(error);
      }
    }
  };

  handleApply = async () => {
    try {
      this.doTask("apply", JSON.stringify(this.mods));
      this.componentUpdate({ mods: this.api("mods"), dirty: false });
    } catch (error) {
      this.showError(error);
    }
  };

  handleCancel = () => {
    this.componentUpdate({ mods: this.api("mods"), dirty: false });
  };

  render() {
    return (
      <div style="flow: vertical; size: *;">
        {this.busy ? (
          <Busy
            text={
              this.log ? this.log[this.log.length - 1].args : "Getting started"
            }
          />
        ) : (
          []
        )}
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
                <strong>{this.mods.filter(m => m.enabled).length} </strong>
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
                {this.dirty ? (
                  <DirtyBar
                    onApply={this.handleApply}
                    onCancel={this.handleCancel}
                  />
                ) : (
                  []
                )}
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
