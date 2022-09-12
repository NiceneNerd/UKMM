export class ModInfo extends Element {
  this(props) {
    this.props = props;
  }

  render() {
    const mod = this.props.mod?.meta;
    return mod ? (
      <div styleset={__DIR__ + "ModInfo.css#ModInfo"}>
        <Row key="Name" val={mod.name} />
        <Row key="Version" val={mod.version.toFixed(2)} />
        <Row key="Category" val={mod.category} />
        <Row key="Author" val={mod.author} />
        {mod.url ? <Row key="Webpage" val={mod.url} /> : []}
        <Long key="Description" val={mod.description} />
        {mod.option_groups?.length > 0 ? (
          <Long
            key="Options"
            val={mod.option_groups.flatMap((group) =>
              group.options.map((opt) => (
                <div>
                  <input
                    key={opt.name}
                    state-disabled={true}
                    type="checkbox"
                    checked={this.props.mod.enabled_options.includes(opt.name)}
                  />
                  {" "}
                  {opt.name}
                </div>
              ))
            )}
          />
        ) : (
          []
        )}
      </div>
    ) : (
      []
    );
  }
}

const Row = ({ key, val }) => (
  <div class="row">
    <div class="label">{key}</div>
    <div class="data">{val}</div>
  </div>
);

const Long = ({ key, val }) => (
  <div class="long">
    <div class="label">{key}</div>
    <div class="data">{val}</div>
  </div>
);
