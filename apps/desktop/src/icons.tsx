// Fluent UI System Icons (Microsoft, MIT): each icon is a file of the app, painted in the colour
// of the text through a CSS mask – no markup is injected and nothing comes from the network.

import home from "@fluentui/svg-icons/icons/home_20_regular.svg";
import search from "@fluentui/svg-icons/icons/search_20_regular.svg";
import database from "@fluentui/svg-icons/icons/database_20_regular.svg";
import lightbulb from "@fluentui/svg-icons/icons/lightbulb_20_regular.svg";
import globeSearch from "@fluentui/svg-icons/icons/globe_search_20_regular.svg";
import settings from "@fluentui/svg-icons/icons/settings_20_regular.svg";
import open from "@fluentui/svg-icons/icons/open_20_regular.svg";
import documentText from "@fluentui/svg-icons/icons/document_text_20_regular.svg";
import copy from "@fluentui/svg-icons/icons/copy_20_regular.svg";
import checkmark from "@fluentui/svg-icons/icons/checkmark_20_regular.svg";
import dismiss from "@fluentui/svg-icons/icons/dismiss_20_regular.svg";
import info from "@fluentui/svg-icons/icons/info_20_regular.svg";
import warning from "@fluentui/svg-icons/icons/warning_20_regular.svg";
import play from "@fluentui/svg-icons/icons/play_20_regular.svg";
import stop from "@fluentui/svg-icons/icons/stop_20_regular.svg";
import history from "@fluentui/svg-icons/icons/history_20_regular.svg";
import shield from "@fluentui/svg-icons/icons/shield_checkmark_20_regular.svg";
import arrowLeft from "@fluentui/svg-icons/icons/arrow_left_20_regular.svg";
import arrowRight from "@fluentui/svg-icons/icons/arrow_right_20_regular.svg";
import arrowSync from "@fluentui/svg-icons/icons/arrow_sync_20_regular.svg";
import add from "@fluentui/svg-icons/icons/document_add_20_regular.svg";
import moon from "@fluentui/svg-icons/icons/weather_moon_20_regular.svg";
import sun from "@fluentui/svg-icons/icons/weather_sunny_20_regular.svg";
import erase from "@fluentui/svg-icons/icons/delete_20_regular.svg";
import people from "@fluentui/svg-icons/icons/people_team_20_regular.svg";
import location from "@fluentui/svg-icons/icons/location_20_regular.svg";
import globe from "@fluentui/svg-icons/icons/globe_20_regular.svg";
import toolbox from "@fluentui/svg-icons/icons/toolbox_20_regular.svg";
import lockClosed from "@fluentui/svg-icons/icons/lock_closed_20_regular.svg";
import lockOpen from "@fluentui/svg-icons/icons/lock_open_20_regular.svg";
import save from "@fluentui/svg-icons/icons/save_20_regular.svg";
import link from "@fluentui/svg-icons/icons/link_20_regular.svg";
import archive from "@fluentui/svg-icons/icons/archive_20_regular.svg";

const ICONS = {
  home,
  search,
  database,
  lightbulb,
  globeSearch,
  settings,
  open,
  documentText,
  copy,
  checkmark,
  dismiss,
  info,
  warning,
  play,
  stop,
  history,
  shield,
  arrowLeft,
  arrowRight,
  arrowSync,
  add,
  moon,
  sun,
  erase,
  people,
  location,
  globe,
  toolbox,
  lockClosed,
  lockOpen,
  save,
  link,
  archive,
};

export type IconName = keyof typeof ICONS;

export function Icon({ name, size = 20 }: { name: IconName; size?: number }) {
  const url = `url("${ICONS[name]}")`;
  return (
    <span
      className="icon"
      aria-hidden="true"
      style={{ width: size, height: size, maskImage: url, WebkitMaskImage: url }}
    />
  );
}
