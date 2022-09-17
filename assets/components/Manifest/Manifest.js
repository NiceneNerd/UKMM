const TreeView = ({ path }) => {
    console.log(path);
    return path.children.length == 0 ? (
        <option>{path.name}</option>
    ) : (
        <option>
            <caption>{path.name}</caption>
            {path.children.map(child => (
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
                    r.result.push({ name, children: r[name].result });
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
