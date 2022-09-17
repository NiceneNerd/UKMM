import { Apply } from "../../icons/Apply";
import { Cancel } from "../../icons/Cancel";

export const DirtyBar = ({ onApply, onCancel }) => (
  <div styleset={__DIR__ + "DirtyBar.css#DirtyBar"}>
    <div class="label">Changes Pending Apply</div>
    <button class="icon" title="Apply Pending Changes" onClick={onApply}>
      <Apply />
    </button>
    <button class="icon danger" title="Cancel Pending Changes" onClick={onCancel}>
      <Cancel />
    </button>
  </div>
);
