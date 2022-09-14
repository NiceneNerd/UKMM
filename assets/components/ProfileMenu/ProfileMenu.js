import { Plus } from "../../icons/Plus";
import { Trash } from "../../icons/Trash";
import { Overflow } from "../../icons/Overflow";

export class ProfileMenu extends Element {
  this(props) {
    this.props = props;
    this.handleDelete = this.handleDelete.bind(this);
  }

  handleDelete() {
    if (Window.this.question("Are you sure you want to delete profile "
      + this.props.currentProfile
      + "?")) {
      prompt("Hey bro");
    }
  }

  render() {
    return (
      <div styleset={__DIR__ + "ProfileMenu.css#ProfileMenu"}>
        <div style="height: *; vertical-align: middle;">Profile:</div>
        <select value={this.props.currentProfile}>
          {this.props.profiles.map((profile) => (
            <option key={profile}>{profile}</option>
          ))}
        </select>
        <button.icon title="Delete Profile" onClick={this.handleDelete}>
          <Trash />
        </button>
        <button.icon title="New Profile">
          <Plus />
        </button>
        <button.icon title="Manage Profiles…">
          <Overflow />
        </button>
      </div>
    );
  }
}
