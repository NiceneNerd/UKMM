const TreeView = ({ path: node }) => {
  return node.children.length == 0 ? (
    <option
      title={`Mods using this file:${Window.this.files[node.path].map(
        m => " " + m.meta.name
      )}`}>
      {node.name}
    </option>
  ) : (
    <option>
      <caption>{node.name}</caption>
      {node.children.map(child => (
        <TreeView key={child.name} path={child} />
      ))}
    </option>
  );
};

class Tree extends Element {
  this(props) {
    this.tree = [];
    let level = { result: this.tree };
    props.paths.forEach(path => {
      path.split("/").reduce((r, name, i, a) => {
        if (!r[name]) {
          r[name] = { result: [] };
          r.result.push({ name, children: r[name].result, path });
        }

        return r[name];
      }, level);
    });
  }

  render() {
    return (
      <select type="tree" styleset={__DIR__ + "Manifest.css#Tree"}>
        {this.tree.map(node => (
          <TreeView key={node.name} path={node} />
        ))}
      </select>
    );
  }
}

export const Manifest = ({ manifest }) => {
  let tree = [];
  if (manifest.content.length > 0) {
    tree.push(...manifest.content.map(f => "Base Files/" + f));
  }
  if (manifest.aoc.length > 0) {
    tree.push(...manifest.aoc.map(f => "DLC Files/" + f));
  }
  return <Tree paths={tree} />;
};
