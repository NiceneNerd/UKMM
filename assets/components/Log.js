export class Log extends Element {
    constructor(props) {
        super(props);
        this.levelColor = this.levelColor.bind(this);
    }

    this(props) {
        this.props = props;
    }

    levelColor(level) {
        switch (level) {
            case "WARN":
                return "gold";
            case "INFO":
                return "forestgreen";
            case "DEBUG":
                return "royalblue";
            case "ERROR":
                return "red";
            default:
                return "color(panel-text)";
        }
    }

    componentDidMount() {
        this.vlist = document.querySelector("#Log").vlist;
    }

    componentDidUpdate() {
        this.vlist.advanceTo(this.props.logs.length - 1);
    }

    render() {
        this.post(this.componentDidUpdate);
        return (
            <div #Log styleset={__DIR__ + "Log.css#Log"} selectable>
                {this.props.logs.map(entry => (
                    <div selectable key={entry.msg}>
                        [<span class="time">{entry.timestamp}</span>{" "}
                        <span class="level" style={`color: ${this.levelColor(entry.level)}`}>{entry.level}</span>]{" "}
                        <span class="msg">{entry.args}</span>
                    </div>
                ))}
            </div>
        );
    }
}
