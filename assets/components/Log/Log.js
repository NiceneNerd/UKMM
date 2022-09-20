export class Log extends Element {
    this(props) {
        this.props = props;
    }

    componentDidMount() {
        this.list = document.querySelector("#Log");
    }

    componentDidUpdate() {
        // this.vlist.advanceTo(this.props.logs.length - 1);
        this.list.scrollTop = this.list.scrollHeight;
    }

    render() {
        this.post(this.componentDidUpdate);
        return (
            <plaintext #Log styleset={__DIR__ + "Log.css#Log"} readonly>
                {this.props.logs.map(entry => (
                    <text key={entry.msg}>
                        [<span class="time">{entry.timestamp}</span>{" "}
                        <span class={"level " + entry.level}>{entry.level}</span>]{" "}
                        <span class="msg">{entry.args}</span>
                    </text>
                ))}
            </plaintext>
        );
    }
}
