import SwiftUI

/// Glass versions of the drawer's theme switcher and settings button, drawn
/// exactly over the (hidden) web buttons; a tap is passed on to them.
struct DrawerButtons: View {
  @ObservedObject var model: CardModel

  var body: some View {
    ZStack(alignment: .topLeading) {
      Color.clear
      ForEach(model.drawerButtons, id: \.id) { b in
        let size = max(40, max(b.w, b.h))
        Button {
          model.button(b.id)
        } label: {
          Image(systemName: b.symbol)
            .font(.system(size: 18, weight: .medium))
            .foregroundStyle(Color.primary)
            .frame(width: size, height: size)
            .contentTransition(.symbolEffect(.replace))
        }
        .buttonStyle(.plain)
        .glassEffect(.regular.interactive(), in: .circle)
        .touchable("drawer-\(b.id)", in: model)
        .position(x: b.x + b.w / 2, y: b.y + b.h / 2)
        .accessibilityLabel(b.id == "theme" ? "Theme" : "Settings")
        .transition(.opacity)
      }
    }
    .ignoresSafeArea()
  }
}

/// The floating glass buttons over the task list: menu (top left), search
/// (top right) and add (bottom right). Hidden while the card or the drawer
/// is open.
struct FloatingButtons: View {
  @ObservedObject var model: CardModel

  var body: some View {
    VStack {
      HStack {
        round("line.3.horizontal", key: "menu", label: "Open menu")
        Spacer()
        round(model.searching ? "xmark" : "magnifyingglass", key: "search", label: model.searching ? "Close search" : "Search")
      }
      .padding(.horizontal, 14)
      Spacer()
      // The + steps aside while the card is open (the card covers it).
      if model.addVisible && !model.isOpen {
      HStack {
        Spacer()
        Button {
          model.button("fab")
        } label: {
          Image(systemName: "plus")
            .font(.title2.weight(.semibold))
            .foregroundStyle(.white)
            .frame(width: 58, height: 58)
        }
        .buttonStyle(.plain)
        .glassEffect(.regular.tint(keelGreen).interactive(), in: .circle)
        .touchable("fab", in: model)
        .accessibilityLabel("Add task")
      }
      .padding(.trailing, 20)
      .padding(.bottom, 12)
      }
    }
  }

  func round(_ icon: String, key: String, label: String) -> some View {
    Button {
      model.button(key)
    } label: {
      Image(systemName: icon)
        .font(.body.weight(.semibold))
        .foregroundStyle(Color.primary)
        .frame(width: 44, height: 44)
    }
    .buttonStyle(.plain)
    .glassEffect(.regular.interactive(), in: .circle)
    .touchable(key, in: model)
    .accessibilityLabel(label)
  }
}
