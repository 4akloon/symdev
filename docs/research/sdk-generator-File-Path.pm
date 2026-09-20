# Stand-in for the core File::Path.
#
# On Windows the core module splits paths with File::Spec::Win32, which knows
# that "\" is a separator; on Linux it does not, so an SDK path arrives as a
# single component and mkpath() degenerates into one mkdir of the whole
# string.  This shim normalises the separator and then does the same job.
# Only mkpath and rmtree are used by the SDK tools.

package File::Path;

require Exporter;
@ISA    = qw(Exporter);
@EXPORT = qw(mkpath rmtree);

sub _norm {
    my $p = shift;
    return $p unless defined $p;
    $p =~ s{\\}{/}g;
    return $p;
}

sub mkpath {
    my $arg = shift;
    my @paths = (ref($arg) eq 'ARRAY') ? @$arg : ($arg);
    my @made;
    foreach my $raw (@paths) {
        next unless defined $raw && length $raw;
        my $path = _norm($raw);
        $path =~ s{/+$}{};
        next unless length $path;
        my @parts = split m{/}, $path, -1;
        my $acc = '';
        for (my $i = 0 ; $i <= $#parts ; $i++) {
            $acc = ($i == 0) ? $parts[$i] : "$acc/$parts[$i]";
            next if $acc eq '';
            next if -d $acc;
            next unless mkdir $acc;
            push @made, $acc;
        }
    }
    return @made;
}

sub _rm {
    my $path = shift;
    my $n    = 0;
    if (-d $path && !-l $path) {
        if (opendir my $dh, $path) {
            my @kids = grep { $_ ne '.' && $_ ne '..' } readdir $dh;
            closedir $dh;
            $n += _rm("$path/$_") for @kids;
        }
        $n++ if rmdir $path;
    } elsif (-e $path || -l $path) {
        $n++ if unlink $path;
    }
    return $n;
}

sub rmtree {
    my $arg = shift;
    my @paths = (ref($arg) eq 'ARRAY') ? @$arg : ($arg);
    my $n = 0;
    foreach my $raw (@paths) {
        next unless defined $raw && length $raw;
        $n += _rm(_norm($raw));
    }
    return $n;
}

1;
