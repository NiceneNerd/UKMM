import { ModList } from "./components/ModList";

export class App extends Element {
    constructor(props) {
        super(props);
        this.props = props;
        this.api = Window.this.xcall("GetApi");
        this.mods = [];
        this.handleToggle = this.handleToggle.bind(this);
        this.handleReorder = this.handleReorder.bind(this);
    }

    componentDidMount() {
        this.componentUpdate({ mods: this.api.mods() });
    }

    handleToggle(mod) {
        console.log(`Toggling ${mod.meta.name}`);
        // let mod = this.mods.find(m => m == mod);
        mod.enabled = !mod.enabled;
        this.componentUpdate({ mods: this.mods });
    }

    handleReorder(oldIdxs, newIdx) {
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
        this.componentUpdate({ mods });
    }

    render() {
        return (
            <div>
                <p>Hello world</p>
                <ModList
                    mods={this.mods}
                    onToggle={this.handleToggle}
                    onReorder={this.handleReorder}
                />
                <button>Testing a button</button>
            </div>
        );
    }
}
