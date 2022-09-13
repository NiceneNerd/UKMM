import { ModList } from "./components/ModList";
import { Log } from "./components/Log";
import { MenuBar } from "./components/MenuBar";
import { Tabs, Tab } from "./components/Tabs";
import { ProfileMenu } from "./components/ProfileMenu";
import { ModInfo } from "./components/ModInfo";
import { Toolbar } from "./components/Toolbar";

export class App extends Element {
  constructor(props) {
    super(props);
    this.props = props;
    this.api = Window.this.xcall("GetApi");
    Window.this.api = this.api;
    this.mods = [];
    this.currentMod = 0;
    this.profiles = [];
    this.currentProfile = "Default";
    this.handleToggle = this.handleToggle.bind(this);
    this.handleReorder = this.handleReorder.bind(this);
    this.handleLog = this.handleLog.bind(this);
    this.handleSelect = this.handleSelect.bind(this);
    Window.this.log = this.handleLog;
    this.dirty = false;
    this.log = [];
  }

  componentDidMount() {
    const mods = this.api.mods();
    const profiles = this.api.profiles();
    const currentProfile = this.api.current_profile();
    this.componentUpdate({ mods, profiles, currentProfile });
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
    this.componentUpdate({ mods });
  }

  handleSelect(index) {
    this.componentUpdate({ currentMod: index });
  }

  handleLog(record) {
    let log = this.log;
    log.push(record);
    this.componentUpdate({ log });
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
              <ModList
                mods={this.mods}
                onToggle={this.handleToggle}
                onReorder={this.handleReorder}
                onSelect={this.handleSelect}
              />
              <splitter />
              <Log logs={this.log} />
            </frameset>
          </div>
          <splitter />
          <Tabs>
            <Tab label="Install">
              <p>todo</p>
            </Tab>
            <Tab label="Mod Info">
              <ModInfo mod={this.mods[this.currentMod]} />
            </Tab>
          </Tabs>
        </frameset>
      </div>
    );
  }
}
