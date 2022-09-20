export class FileInput extends Element {
  constructor(props) {
    super(props);
  }

  this(props) {
    this.props = props;
  }

  render() {
    return (
      <div styleset={__DIR__ + "File.css#File"}>
        <input type="text" value={this.props.value} onChange={this.props.onChange} />
        <button
          onClick={() => {
            const res = Window.this.selectFolder();
            if (res) this.props.onChange(res);
          }}
        >
          Browse…
        </button>
      </div>
    );
  }
}
