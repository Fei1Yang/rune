import 'dart:async';

import 'package:fluent_ui/fluent_ui.dart';
import 'package:go_router/go_router.dart';
import 'package:player/widgets/start_screen/providers/start_screen_layout_manager.dart';
import 'package:provider/provider.dart';

import '../../config/animation.dart';

import '../../messages/album.pb.dart';
import '../../messages/artist.pb.dart';
import '../../messages/library_home.pb.dart';

import '../../screens/albums/albums_list.dart';
import '../../screens/artists/artists_list.dart';

import '../../widgets/smooth_horizontal_scroll.dart';
import '../../widgets/start_screen/start_group.dart';
import '../../widgets/start_screen/start_screen.dart';

class LibraryHomeListView extends StatefulWidget {
  final String libraryPath;
  final StartScreenLayoutManager layoutManager;

  const LibraryHomeListView(
      {super.key, required this.libraryPath, required this.layoutManager});

  @override
  LibraryHomeListState createState() => LibraryHomeListState();
}

class LibraryHomeListState extends State<LibraryHomeListView> {
  Future<List<Group<dynamic>>>? summary;

  @override
  void initState() {
    setState(() {
      summary = fetchSummary();
    });

    super.initState();
  }

  Future<List<Group<dynamic>>> fetchSummary() async {
    final fetchLibrarySummary = FetchLibrarySummaryRequest();
    fetchLibrarySummary.sendSignalToRust(); // GENERATED

    final rustSignal = await LibrarySummaryResponse.rustSignalStream.first;
    final librarySummary = rustSignal.message;

    Timer(Duration(milliseconds: gridAnimationDelay),
        () => widget.layoutManager.playAnimations());

    return [
      Group<Album>(groupTitle: "Albums", items: librarySummary.albums),
      Group<Artist>(groupTitle: "Artists", items: librarySummary.artists)
    ];
  }

  @override
  Widget build(BuildContext context) {
    return FutureBuilder<List<Group<dynamic>>>(
      future: summary,
      builder: (context, snapshot) {
        if (snapshot.connectionState == ConnectionState.waiting) {
          return Container();
        } else if (snapshot.hasError) {
          return Center(child: Text('Error: ${snapshot.error}'));
        } else if (!snapshot.hasData || snapshot.data!.isEmpty) {
          return const Center(child: Text('No data available'));
        } else {
          return Container(
            alignment: Alignment.centerLeft,
            child: SmoothHorizontalScroll(
              builder: (context, scrollController) => SingleChildScrollView(
                scrollDirection: Axis.horizontal,
                controller: scrollController,
                child: Row(
                  mainAxisAlignment: MainAxisAlignment.start,
                  children: snapshot.data!.map((item) {
                    if (item is Group<Album>) {
                      return StartGroup<Album>(
                        groupIndex: 0,
                        groupTitle: item.groupTitle,
                        items: item.items,
                        groupLayoutVariation:
                            StartGroupGroupLayoutVariation.stacked,
                        gridLayoutVariation:
                            StartGroupGridLayoutVariation.square,
                        gapSize: 12,
                        onTitleTap: () => {context.push('/albums')},
                        itemBuilder: (BuildContext context, Album item) =>
                            AlbumItem(album: item),
                      );
                    } else if (item is Group<Artist>) {
                      return StartGroup<Artist>(
                        groupIndex: 1,
                        groupTitle: item.groupTitle,
                        items: item.items,
                        groupLayoutVariation:
                            StartGroupGroupLayoutVariation.stacked,
                        gridLayoutVariation:
                            StartGroupGridLayoutVariation.square,
                        gapSize: 12,
                        onTitleTap: () => {context.push('/artists')},
                        itemBuilder: (BuildContext context, Artist item) =>
                            ArtistItem(artist: item),
                      );
                    } else {
                      return Container();
                    }
                  }).toList(),
                ),
              ),
            ),
          );
        }
      },
    );
  }
}
