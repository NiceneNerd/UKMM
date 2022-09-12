import { ModList } from "./components/ModList";
import { Log } from "./components/Log";
import { MenuBar } from "./components/MenuBar";
import { Tabs, Tab } from "./components/Tabs";

export class App extends Element {
  constructor(props) {
    super(props);
    this.props = props;
    this.api = Window.this.xcall("GetApi");
    this.mods = [];
    this.handleToggle = this.handleToggle.bind(this);
    this.handleReorder = this.handleReorder.bind(this);
    this.handleLog = this.handleLog.bind(this);
    Window.this.log = this.handleLog;
    this.dirty = false;
    this.log = [];
  }

  componentDidMount() {
    this.componentUpdate({ mods: this.api.mods() });
  }

  handleToggle(mod) {
    mod.enabled = !mod.enabled;
    this.componentUpdate({ mods: this.mods, dirty: true });
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

  handleLog(record) {
    let log = this.log;
    log.push(record);
    this.componentUpdate({ log });
  }

  render() {
    return (
      <div style="flow: vertical; size: *;">
        <MenuBar />
        <frameset cols="*,33.33%" style="size: *;">
          <div style="size: *;">
            <frameset rows="*,15%" style="size: *;">
              <ModList
                mods={this.mods}
                onToggle={this.handleToggle}
                onReorder={this.handleReorder}
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
              <p>Hiro</p>
            </Tab>
          </Tabs>
        </frameset>
      </div>
    );
  }
}
