use std::sync::Arc;

use druid::{
    widget::{CrossAxisAlignment, Flex, Label, LineBreaking, List, ViewSwitcher},
    LensExt, LocalizedString, Menu, MenuItem, Selector, Size, UnitPoint, Widget, WidgetExt,
};

use crate::{
    cmd,
    data::{
        Album, AlbumDetail, AlbumLink, AppState, ArtistLink, Cached, Ctx, Library, Nav, Playable,
        PlaybackOrigin, WithCtx,
    },
    ui::playable::PlayableIter,
    webapi::WebApi,
    widget::{icons, Async, MyWidgetExt, RemoteImage},
};

use super::{artist, library, playable, theme, track, utils};

pub const LOAD_DETAIL: Selector<AlbumLink> = Selector::new("app.album.load-detail");

pub fn detail_widget() -> impl Widget<AppState> {
    Async::new(
        utils::spinner_widget,
        loaded_detail_widget,
        utils::error_widget,
    )
    .lens(
        Ctx::make(
            AppState::common_ctx,
            AppState::album_detail.then(AlbumDetail::album),
        )
        .then(Ctx::in_promise()),
    )
    .on_command_async(
        LOAD_DETAIL,
        |d| WebApi::global().get_album(&d.id),
        |_, data, d| data.album_detail.album.defer(d),
        |_, data, r| data.album_detail.album.update(r),
    )
}

fn loaded_detail_widget() -> impl Widget<WithCtx<Cached<Arc<Album>>>> {
    let album_cover = rounded_cover_widget(theme::grid(10.0))
        .lens(Ctx::data())
        .context_menu(album_ctx_menu);

    let album_artists = List::new(artist::link_widget).lens(Album::artists.in_arc());

    let album_date = Label::dynamic(|album: &Arc<Album>, _| album.release())
        .with_text_size(theme::TEXT_SIZE_SMALL);

    let album_label = Label::raw()
        .with_line_break_mode(LineBreaking::WordWrap)
        .with_text_size(theme::TEXT_SIZE_SMALL)
        .with_text_color(theme::PLACEHOLDER_COLOR)
        .lens(Album::label.in_arc());

    let album_info = Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(album_artists)
        .with_default_spacer()
        .with_child(album_date)
        .with_default_spacer()
        .with_child(album_label)
        .padding(theme::grid(1.0));

    let album_top = Flex::row()
        .with_spacer(theme::grid(4.2))
        .with_child(album_cover)
        .with_default_spacer()
        .with_child(album_info.lens(Ctx::data()));

    let album_tracks = playable::list_widget(playable::Display {
        track: track::Display {
            number: true,
            title: true,
            artist: true,
            ..track::Display::empty()
        },
    });

    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_spacer(theme::grid(1.0))
        .with_child(album_top)
        .with_spacer(theme::grid(1.0))
        .with_child(album_tracks)
        .lens(Ctx::map(Cached::data))
}

fn cover_widget(size: f64) -> impl Widget<Arc<Album>> {
    RemoteImage::new(utils::placeholder_widget(), move |album: &Arc<Album>, _| {
        album.image(size, size).map(|image| image.url.clone())
    })
    .fix_size(size, size)
}

fn rounded_cover_widget(size: f64) -> impl Widget<Arc<Album>> {
    cover_widget(size).clip(Size::new(size, size).to_rounded_rect(4.0))
}

pub fn album_widget(horizontal: bool) -> impl Widget<WithCtx<Arc<Album>>> {
    let (album_cover_size, album_name_layout) = if horizontal {
        (16.0, Flex::column())
    } else {
        (6.0, Flex::row())
    };
    let album_cover = rounded_cover_widget(theme::grid(album_cover_size));

    let album_name = album_name_layout
        .with_child(
            Label::raw()
                .with_font(theme::UI_FONT_MEDIUM)
                .with_line_break_mode(LineBreaking::Clip)
                .lens(Album::name.in_arc()),
        )
        .with_spacer(theme::grid(0.5))
        .with_child(ViewSwitcher::new(
            |album: &Arc<Album>, _| album.has_explicit(),
            |selector: &bool, _, _| match selector {
                true => icons::EXPLICIT.scale(theme::ICON_SIZE_TINY).boxed(),
                false => Box::new(Flex::column()),
            },
        ));

    let album_artists = List::new(|| {
        Label::raw()
            .with_text_size(theme::TEXT_SIZE_SMALL)
            .with_line_break_mode(LineBreaking::WordWrap)
            .lens(ArtistLink::name)
    })
    .horizontal()
    .with_spacing(theme::grid(1.0))
    .lens(Album::artists.in_arc());

    let album_date = Label::<Arc<Album>>::dynamic(|album, _| album.release_year())
        .with_line_break_mode(LineBreaking::WordWrap)
        .with_text_size(theme::TEXT_SIZE_SMALL)
        .with_text_color(theme::PLACEHOLDER_COLOR);

    let album_layout = if horizontal {
        Flex::column()
            .with_child(album_cover)
            .with_default_spacer()
            .with_child(
                Flex::column()
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .with_child(album_name)
                    .with_spacer(1.0)
                    .with_child(album_artists)
                    .with_spacer(1.0)
                    .with_child(album_date)
                    .align_horizontal(UnitPoint::CENTER)
                    .align_vertical(UnitPoint::TOP)
                    .fix_size(theme::grid(16.0), theme::grid(8.0)),
            )
            .align_left()
    } else {
        Flex::row()
            .with_child(album_cover)
            .with_default_spacer()
            .with_flex_child(
                Flex::column()
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .with_child(album_name)
                    .with_spacer(1.0)
                    .with_child(album_artists)
                    .with_spacer(1.0)
                    .with_child(album_date),
                1.0,
            )
            .align_left()
    };

    album_layout
        .padding(theme::grid(1.0))
        .lens(Ctx::data())
        .link()
        .rounded(theme::BUTTON_BORDER_RADIUS)
        .on_left_click(|ctx, _, album, _| {
            ctx.submit_command(cmd::NAVIGATE.with(Nav::AlbumDetail(album.data.link(), None)));
        })
        .context_menu(album_ctx_menu)
}

fn album_ctx_menu(album: &WithCtx<Arc<Album>>) -> Menu<AppState> {
    album_menu(&album.data, &album.ctx.library)
}

fn album_menu(album: &Arc<Album>, library: &Arc<Library>) -> Menu<AppState> {
    let mut menu = Menu::empty();

    for artist_link in &album.artists {
        let more_than_one_artist = album.artists.len() > 1;
        let title = if more_than_one_artist {
            LocalizedString::new("menu-item-show-artist-name")
                .with_placeholder(format!("Go to Artist \"{}\"", artist_link.name))
        } else {
            LocalizedString::new("menu-item-show-artist").with_placeholder("Go to Artist")
        };
        menu = menu.entry(
            MenuItem::new(title)
                .command(cmd::NAVIGATE.with(Nav::ArtistDetail(artist_link.to_owned()))),
        );
    }

    menu = menu.entry(
        MenuItem::new(
            LocalizedString::new("menu-item-copy-link").with_placeholder("Copy Link to Album"),
        )
        .command(cmd::COPY.with(album.url())),
    );

    menu = menu.separator();

    if library.contains_album(album) {
        menu = menu.entry(
            MenuItem::new(
                LocalizedString::new("menu-item-remove-from-library")
                    .with_placeholder("Remove Album from Library"),
            )
            .command(library::UNSAVE_ALBUM.with(album.link())),
        );
    } else {
        menu = menu.entry(
            MenuItem::new(
                LocalizedString::new("menu-item-save-to-library")
                    .with_placeholder("Save Album to Library"),
            )
            .command(library::SAVE_ALBUM.with(album.clone())),
        );
    }

    menu
}

impl PlayableIter for Arc<Album> {
    fn origin(&self) -> PlaybackOrigin {
        PlaybackOrigin::Album(self.link())
    }

    fn for_each(&self, mut cb: impl FnMut(Playable, usize)) {
        for (position, track) in self.clone().into_tracks_with_context().iter().enumerate() {
            cb(Playable::Track(track.clone()), position);
        }
    }

    fn count(&self) -> usize {
        self.tracks.len()
    }
}
