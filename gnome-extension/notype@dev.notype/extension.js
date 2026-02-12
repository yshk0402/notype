const Gio = imports.gi.Gio;
const Meta = imports.gi.Meta;
const Shell = imports.gi.Shell;
const Main = imports.ui.main;
const ExtensionUtils = imports.misc.extensionUtils;

const BUS_NAME = 'dev.notype.app';
const OBJECT_PATH = '/dev/notype/app';
const INTERFACE_NAME = 'dev.notype.app';
const KEY_NAME = 'toggle-recording';

let _settings = null;

function _notify(message) {
  Main.notify('notype', message);
}

function _callToggleRecording() {
  Gio.DBus.session.call(
    BUS_NAME,
    OBJECT_PATH,
    INTERFACE_NAME,
    'ToggleRecording',
    null,
    null,
    Gio.DBusCallFlags.NONE,
    1500,
    null,
    (_conn, result) => {
      try {
        Gio.DBus.session.call_finish(result);
      } catch (err) {
        _notify(`Toggle failed: ${err.message}`);
      }
    }
  );
}

function init() {
}

function enable() {
  _settings = ExtensionUtils.getSettings('org.gnome.shell.extensions.notype');
  Main.wm.addKeybinding(
    KEY_NAME,
    _settings,
    Meta.KeyBindingFlags.NONE,
    Shell.ActionMode.ALL,
    () => {
      _callToggleRecording();
    }
  );
}

function disable() {
  Main.wm.removeKeybinding(KEY_NAME);
  _settings = null;
}
