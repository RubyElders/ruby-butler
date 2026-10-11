module MicroGems
  class Package
    class TarInvalidError < StandardError; end
    class TooLongFileName < StandardError; end

    module TarBlock
      def new(*args)
        object = super
        return object unless block_given?

        begin
          yield object
        ensure
          object.close
        end
        nil
      end
    end

    class TarHeader
      FIELDS = { name: 100, mode: 8, uid: 8, gid: 8, size: 12, mtime: 12,
                 checksum: 8, typeflag: 1, linkname: 100, magic: 6, version: 2,
                 uname: 32, gname: 32, devmajor: 8, devminor: 8, prefix: 155 }.freeze
      NUMBERS = %i[mode uid gid size mtime checksum devmajor devminor].freeze
      attr_accessor(*FIELDS.keys)

      def initialize(values)
        defaults = { name: '', mode: 0, uid: 0, gid: 0, size: 0, mtime: 0,
                     checksum: 0, typeflag: '0', linkname: '', magic: "ustar\0",
                     version: '00', uname: '', gname: '', devmajor: 0, devminor: 0, prefix: '' }
        defaults.merge(values).each { |key, value| public_send("#{key}=", value) }
        self.mtime = mtime.to_i
      end

      def self.from(io)
        bytes = io.read(512)
        raise TarInvalidError, 'Truncated tar header' unless bytes && bytes.bytesize == 512
        return new(name: '', empty: true) if bytes == "\0" * 512

        values = {}
        offset = 0
        FIELDS.each do |key, width|
          value = bytes.byteslice(offset, width)
          offset += width
          if NUMBERS.include?(key)
            value = value.delete("\0 ")
            raise TarInvalidError, "Invalid tar number: #{key}" unless value.match?(/\A[0-7]*\z/)

            value = value.empty? ? 0 : value.to_i(8)
          else
            value = value.split("\0", 2).first || ''
          end
          values[key] = value
        end
        checked = bytes.dup
        checked[148, 8] = ' ' * 8
        raise TarInvalidError, 'Invalid tar checksum' unless checked.bytes.sum == values[:checksum]

        new(values)
      end

      attr_writer :empty

      def empty? = !!@empty
      def full_name = prefix.empty? ? name : "#{prefix}/#{name}"

      def to_s
        bytes = FIELDS.map do |key, width|
          value = public_send(key)
          if key == :checksum
            ' ' * width
          elsif NUMBERS.include?(key)
            value = Integer(value).to_s(8)
            raise TarInvalidError, "Tar number too large: #{key}" if value.size >= width || value.start_with?('-')

            value.rjust(width - 1, '0') + "\0"
          else
            value = value.to_s.b
            raise TooLongFileName, "Tar field too long: #{key}" if value.bytesize > width

            value.ljust(width, "\0")
          end
        end.join.ljust(512, "\0")
        self.checksum = bytes.bytes.sum
        bytes[148, 8] = format("%06o\0 ", checksum)
        bytes
      end
    end

    class TarReader
      include Enumerable
      extend TarBlock

      class Entry
        attr_reader :header

        def initialize(header, io)
          @header, @io, @read, @closed = header, io, 0, false
        end

        def full_name = header.full_name
        def size = header.size
        def file? = ['0', ''].include?(header.typeflag)
        def directory? = header.typeflag == '5'
        def symlink? = header.typeflag == '2'
        def closed? = @closed
        def eof? = @read == size

        def read(length = nil)
          raise IOError, 'Closed tar entry' if closed?
          raise ArgumentError, 'Negative read size' if length && length < 0
          return nil if length && length > 0 && eof?

          count = length ? [length, size - @read].min : size - @read
          value = @io.read(count)
          raise TarInvalidError, 'Truncated tar entry' unless value && value.bytesize == count

          @read += count
          value
        end

        def close
          return if closed?

          read([size - @read, 16384].min) until eof?
          padding = (-size) % 512
          value = @io.read(padding)
          raise TarInvalidError, 'Truncated tar padding' unless value && value.bytesize == padding

          @closed = true
        end
      end

      def initialize(io)
        @io, @start = io, io.pos
      end

      def close; end
      def rewind = @io.pos = @start

      def each
        return enum_for(__method__) unless block_given?

        until @io.eof?
          header = TarHeader.from(@io)
          break if header.empty?

          entry = Entry.new(header, @io)
          begin
            yield entry
          ensure
            entry.close
          end
        end
      end
      alias each_entry each
    end
  end
end
